# Rekhta Crawler — Plan

> A crawler that discovers Urdu/Hindi PDF books on `rekhta.org`, downloads
> them, and enqueues them into the existing **5-stage admin ingestion
> pipeline** (Upload → Process → Review → Edit → Publish) so they appear
> as new books in the library after human review.
>
> **Status:** plan only. No crawler code exists yet. This document is the
> design north-star per the project's "plan docs are authoritative" rule.

---

## 0. Before you build — legal, ethical, scope

This section is a hard gate. None of the rest of the plan ships until
these are answered in writing and the answers are linked from here.

| Question | Default answer (override only with evidence) |
|---|---|
| Does `rekhta.org/robots.txt` permit our paths? | Honor it. If it disallows `/ebooks/`, the crawler does not touch them. |
| Are the works we target out of copyright in our jurisdiction? | **Public-domain only by default** — Ghalib, Mir, Iqbal, Daagh, Zauq, etc. Living/recent authors are *opt-in per-author* with a written rights note. |
| Are the *PDF scans themselves* a separate copyrightable work? | Treat scans of PD works as PD unless rekhta has added editorial apparatus (intros, notes, glossaries). If they have, prefer the underlying text after OCR, not the scan as-shipped. |
| Does our use match rekhta's ToS? | If not, prefer Internet Archive / Hathitrust / Rekhta Foundation's *explicitly free* download buttons. The crawler should be **source-pluggable** — `rekhta` is one source, not the only one. |
| Will we honor takedowns? | Yes. There is a `crawl_target.takedown_at` column from day one (§5) and an admin mutation to set it (§9). Once set, the book is unpublished and the source is blacklisted from re-import. |
| Identification | The crawler ships a stable `User-Agent: MusanifCrawler/0.1 (+contact)`; no header spoofing, no captcha bypass, no rotating IPs. |

If any of the first four answers is "no" or "unclear" for a given book,
the crawler **does not download it**. The default disposition for an
unknown work is *skip*, logged, not *try and let review catch it*.

---

## 1. Where this fits in the existing system

```
                 ┌────────────────────────────────────────────────┐
   NEW           │            5-stage admin pipeline              │
   ↓             │  (already designed — merk-ingest + admin GQL)  │
                 │                                                │
┌─────────────┐  │   1. Upload    2. Process   3. Review          │
│ merk-crawl  │──▶─► creates ─►   parse/OCR ─► chapter detect ─►  │
│  (new)      │  │   draft +     summarize    + AI metadata       │
└─────────────┘  │   uploads     embed                            │
                 │                                                │
                 │   4. Edit      5. Publish                      │
                 │   human fixes  pre-flight checks → book row    │
                 └────────────────────────────────────────────────┘
```

**Key design decision:** the crawler is a **source**, not a stage. It
produces the same artifact a human dropping a PDF into the admin upload
zone would — a stored blob + a created ingestion `job` row with
pre-populated metadata hints. Everything from §1 (Upload) onwards is
unchanged.

The crawler **does not** write directly into `book` / `chapter` /
`author`. It writes into `crawl_*` provenance tables (§5) and calls the
existing admin upload + job-create surface. Review/Edit/Publish stays a
human-in-the-loop gate; the crawler never auto-publishes.

This keeps three properties:

1. **One ingestion path.** Whether a PDF comes from a human or the
   crawler, it goes through the same Process/Review/Edit/Publish steps,
   gets the same OCR + AI metadata, and emits the same `JobEvent`s.
2. **No duplicated parsing.** The crawler doesn't try to read PDFs —
   `merk-ingest`'s Stage 2 already does that with `lopdf`.
3. **Human gate is preserved.** A pile of crawled books queues up in
   `Review` and waits for an admin. There is no path from "crawler
   started" to "published book" without explicit approval.

---

## 2. New crate: `merk-crawl`

Sibling crate, same workspace shape as `merk-ingest`. Lives at
`/home/usairim/Documents/projects/personal/Musanif-e-Rekhta/merk-crawl/`.

```
merk-crawl/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs              ← trait `Source`, `Crawler` orchestrator, error type
    ├── http.rs             ← reqwest client with politeness + robots.txt + UA
    ├── robots.rs           ← parse + cache /robots.txt (per host)
    ├── ratelimit.rs        ← token-bucket per host, jittered
    ├── store.rs            ← trait `CrawlStore` (matches merk-ingest's JobStore pattern)
    ├── handoff.rs          ← calls into admin upload + createIngestJob
    ├── sources/
    │   ├── mod.rs          ← `pub trait Source`
    │   ├── rekhta.rs       ← the rekhta.org implementation
    │   └── archive_org.rs  ← optional second source (also a sanity check
    │                          that Source isn't accidentally rekhta-shaped)
    └── bin/
        └── merk_crawl.rs   ← CLI entry point (see §10)
```

### 2.1 The `Source` trait

```rust
#[async_trait]
pub trait Source: Send + Sync {
    /// Stable identifier — written into crawl_source.kind. Never changes.
    fn id(&self) -> &'static str; // e.g. "rekhta"

    /// Yields book candidates. Implementation paginates internally; the
    /// stream ends when the source has no more candidates for the run's
    /// query/filter.
    async fn discover(&self, q: &Query) -> BoxStream<'_, Candidate>;

    /// Resolve one candidate into a downloadable item — metadata + the
    /// URL of the PDF. Returns Skip with a reason if the candidate is
    /// not eligible (paywalled, no PDF, non-PD without override).
    async fn resolve(&self, c: &Candidate) -> Result<Resolved, ResolveSkip>;
}
```

`Candidate` is the cheapest thing the listing page yields — usually
`{ source_url, title, author_name_raw }`. `Resolved` is everything
needed to feed Upload — `{ pdf_url, title, author_name_raw,
language_hint, year_hint, isbn_hint, cover_url, source_url }`.

Skip reasons are enumerated (`PaywallDetected`, `NoPdf`,
`NotPublicDomain`, `RobotsDisallowed`, `AlreadyImported`) so we can
report skip-rate per source in metrics.

### 2.2 The orchestrator

Same shape as `merk-ingest::Orchestrator`:

```rust
pub struct Crawler {
    pub source: Arc<dyn Source>,
    pub store: Arc<dyn CrawlStore>,
    pub http: Arc<HttpClient>,         // shared, polite, UA-tagged
    pub handoff: Arc<dyn Handoff>,     // calls admin upload + createIngestJob
}
```

`Crawler::run(query)` walks the discover stream, calls `resolve`,
deduplicates against `crawl_target`, downloads, hands off, persists a
`crawl_run` row. On error it records and continues — one bad book does
not kill a run.

---

## 3. Crawl phases for one book

```
discover ──► dedup-check ──► resolve ──► robots-check ──► fetch PDF ──► hash ──► handoff
                                                                                   │
                                                                                   ▼
                                                          admin upload + createIngestJob
                                                                  (already exists)
```

| Phase | What it does | Failure mode |
|---|---|---|
| **discover** | Walks rekhta's listing pages (poet pages, e-book index). Yields `Candidate`s with `source_url`. | Network → retry w/ backoff. HTML drift → metric `crawl_discover_parse_error` per selector; alert if >1% of pages. |
| **dedup-check** | `crawl_target` lookup by `source_url`. If present and not `failed`, skip. | None — fast path. |
| **resolve** | Fetches the book's detail page, finds PDF URL + metadata. | Skip if no PDF link / paywalled / non-PD. |
| **robots-check** | Consults cached `/robots.txt` for both the detail-page host and the PDF host (often a CDN). | Skip with reason `RobotsDisallowed`. |
| **fetch PDF** | Streams the PDF to `merk-blob-store` as `crawl/<source>/<sha>.pdf`. Records `Content-Length`, `ETag`, `Last-Modified`. | Resume on `Range`. Retry on 5xx with exponential backoff + full jitter. |
| **hash** | SHA-256 over the file. Used as the canonical idempotency key (§7). | None. |
| **handoff** | POST to admin upload finalize endpoint to register the blob, then GraphQL `createIngestJob` with `kind=crawler`, source metadata in `hints` JSON. | If the upload finalize fails, mark `crawl_target` as `error`, do not retry automatically — admin must requeue. |

---

## 4. Politeness & resilience

Non-negotiable defaults (configurable in `merk-crawl/config.toml` but
not loosenable beyond the values shown):

- **Concurrency**: max 2 in-flight requests per host. Period.
- **Rate**: token bucket of 30 req/min per host, jittered.
- **Backoff**: full-jitter exponential, cap 5 min, 6 attempts max per
  URL.
- **Conditional GET**: re-discovery uses `If-None-Match` /
  `If-Modified-Since` from the prior `crawl_target` row. We do not
  re-download an unchanged PDF.
- **Resumability**: `crawl_run` rows are checkpointed every N
  candidates; restarting a crashed run picks up at the last checkpoint.
- **Robots cache**: per-host, TTL 24h. Stale fetch is non-blocking — if
  refresh fails, keep using the cached copy and log.
- **Pause flag**: `crawl_source.paused` honored on every `discover`
  yield. Setting it from the admin GraphQL surface (§9) gracefully
  drains in-flight work.

---

## 5. Schema additions

Three new tables, sibling to the existing 25 in
[`docs/schema/01_entities.md`](schema/01_entities.md). All `crawl_*`
tables are operator-facing; they do not appear in public reads.

### `crawl_source`
| Field | Type | Notes |
|---|---|---|
| `id` | record | |
| `kind` | string | `rekhta`, `archive_org`, … — matches `Source::id()`; UNIQUE |
| `base_url` | string | e.g. `https://www.rekhta.org` |
| `enabled` | bool | |
| `paused` | bool | honored by the crawler on next yield |
| `default_query` | option\<object\> | filter to apply if a run doesn't supply one |
| `created_at` | datetime | READONLY |
| `updated_at` | datetime | auto |

### `crawl_run`
| Field | Type | Notes |
|---|---|---|
| `id` | record | |
| `source` | record\<crawl_source\> | |
| `query` | object | the `Query` this run was launched with |
| `started_at` | datetime | READONLY |
| `finished_at` | option\<datetime\> | |
| `status` | string | `running \| completed \| failed \| cancelled` |
| `candidates_seen` | int | |
| `candidates_skipped` | int | |
| `targets_downloaded` | int | |
| `targets_handed_off` | int | |
| `last_checkpoint` | option\<object\> | opaque resume cursor |

### `crawl_target`
| Field | Type | Notes |
|---|---|---|
| `id` | record | |
| `source` | record\<crawl_source\> | |
| `source_url` | string | UNIQUE per source |
| `pdf_url` | option\<string\> | filled in after `resolve` |
| `pdf_sha256` | option\<string\> | filled in after `fetch` — global UNIQUE |
| `pdf_etag` | option\<string\> | for conditional GET |
| `pdf_last_modified` | option\<datetime\> | for conditional GET |
| `title_raw` | option\<string\> | source-side title before normalization |
| `author_name_raw` | option\<string\> | |
| `language_hint` | option\<string\> | |
| `year_hint` | option\<int\> | |
| `cover_url` | option\<string\> | |
| `hints_json` | option\<object\> | everything else the source returned |
| `status` | string | `new \| skipped \| downloaded \| handed_off \| failed \| takedown` |
| `skip_reason` | option\<string\> | enum from `Source::resolve` |
| `ingest_job` | option\<record\<job\>\> | set after handoff |
| `book` | option\<record\<book\>\> | set when ingest publishes — joins back to the catalog |
| `takedown_at` | option\<datetime\> | non-null = removed; blocks re-import |
| `error` | option\<string\> | last error string for `failed` |
| `attempts` | int | |
| `created_at` | datetime | READONLY |
| `updated_at` | datetime | auto |

**Indexes:** `(source, source_url)` UNIQUE, `pdf_sha256` UNIQUE,
`status` (for queues), `created_at` (for activity feed).

---

## 6. Author + book matching

The catalog already has `author.slug` (UNIQUE) and the
`wrote: author → book` graph relation. The crawler must not
double-create authors.

**Resolution order during handoff** (the crawler does this *before*
calling `createIngestJob` so the human reviewer in stage 3 starts
warm):

1. Exact match on `author.slug` after slugifying
   `author_name_raw`.
2. Fuzzy match on `author.name` (Levenshtein ≤ 2 in NFD-normalized
   form, after stripping Urdu honorifics — "Mirza", "Maulana",
   "Hakeem").
3. If multiple matches, the crawler does **not** pick — it stores both
   candidates in `crawl_target.hints_json.author_candidates` and lets
   the admin pick in stage 3.
4. If no match, no author is created — the admin creates one (or
   confirms "create from raw name") at review time.

This is deliberately conservative. Stage-3 review already has an
author-resolution UI (see `admin/authors.rs` in
[`musanif/docs/api-integration-plan.md`](../../musanif/docs/api-integration-plan.md)
§4); the crawler feeds it, doesn't replace it.

---

## 7. Idempotency & dedup

Three keys, layered:

1. **`crawl_target.source_url`** — "did we already see this listing
   page?" Cheapest; lookup before any HTTP.
2. **`crawl_target.pdf_sha256`** — "did we already download this exact
   file from anywhere?" Catches cross-source duplicates (the same PDF
   hosted on rekhta and archive.org). Lookup after download.
3. **`book.isbn`** (existing, UNIQUE) — final safety net at publish
   time; ingest's pre-flight already enforces it.

Re-running a crawl with the same query is safe: every candidate hits
key 1 and is skipped instantly. The only way to force a re-download is
the admin "requeue" mutation, which clears `pdf_sha256` and bumps
`attempts`.

---

## 8. Where the seams sit (file-level)

### In merk (backend)
- New: `merk-crawl/` crate (above).
- New: migration adding the three `crawl_*` tables.
- New: GraphQL admin module `admin/crawl.rs` exposing the queries and
  mutations in §9. Uses the same RBAC guard as the rest of
  `admin/*` — admin role required.
- **Reused** (no changes): `merk-blob-store` for the PDF blobs;
  `merk-ingest::Orchestrator` for everything after handoff;
  `merk-events::JobEvent` because handoff just creates a normal job.
- **Reused** (no changes): existing REST upload finalize endpoint —
  the crawler is just a non-browser client of the same endpoint.

### In musanif (frontend)
- New admin view: "Sources" sub-page under the existing admin sidebar
  (per [`musanif/docs/ui-implementation-plan.md`](../../musanif/docs/ui-implementation-plan.md)
  §2.5). Lists sources, runs, and pending targets.
- **Reused**: the Ingestion queue / Library views in the admin shell
  already render `job` rows; crawler-originated jobs appear there
  automatically with `kind=crawler` distinguishing them.

The crawler binary runs out-of-process (a separate `merk_crawl` CLI /
systemd unit). It is **not** linked into the merk web server; the only
contact point is HTTP (admin upload finalize + GraphQL).

---

## 9. Admin GraphQL surface (`admin/crawl.rs`)

Additions to [`schema/05_graphql.md`](schema/05_graphql.md). All
admin-guarded.

```graphql
type CrawlSource {
  id:        ID!
  kind:      String!
  baseUrl:   String!
  enabled:   Boolean!
  paused:    Boolean!
}

type CrawlRun {
  id:                 ID!
  sourceId:           String!
  startedAt:          String!
  finishedAt:         String
  status:             String!
  candidatesSeen:     Int!
  candidatesSkipped:  Int!
  targetsDownloaded:  Int!
  targetsHandedOff:   Int!
}

type CrawlTarget {
  id:             ID!
  sourceId:       String!
  sourceUrl:      String!
  titleRaw:       String
  authorNameRaw:  String
  status:         String!
  skipReason:     String
  pdfSha256:      String
  ingestJobId:    String
  bookId:         String
  takedownAt:     String
  attempts:       Int!
  createdAt:      String!
}

extend type Query {
  crawlSources:                                        [CrawlSource!]!
  crawlRuns(sourceId: ID, limit: Int, offset: Int):    [CrawlRun!]!
  crawlTargets(
    sourceId: ID,
    status:   String,
    q:        String,
    limit:    Int, offset: Int,
  ): [CrawlTarget!]!
  crawlTarget(id: ID!): CrawlTarget
}

extend type Mutation {
  upsertCrawlSource(input: UpsertCrawlSourceInput!):   CrawlSource!
  setCrawlSourcePaused(id: ID!, paused: Boolean!):     CrawlSource!
  startCrawlRun(sourceId: ID!, query: JSON):           CrawlRun!
  cancelCrawlRun(id: ID!):                             CrawlRun!
  requeueCrawlTarget(id: ID!):                         CrawlTarget!
  blacklistCrawlTarget(id: ID!, reason: String!):      CrawlTarget!
  recordTakedown(id: ID!):                             CrawlTarget!
}
```

`JobEvents` (already designed for the Process screen) carries
crawler-originated jobs unchanged — no new subscription needed.

---

## 10. CLI surface (`merk_crawl`)

For ops / one-shot use without the admin app open:

```
merk_crawl source list
merk_crawl source pause   <kind>
merk_crawl source resume  <kind>
merk_crawl run start      <kind> [--query path/to/query.json]
merk_crawl run watch      <run-id>     # streams candidates_seen / skipped / handed_off
merk_crawl target requeue <target-id>
merk_crawl target takedown <target-id>
```

The CLI is a thin wrapper over the GraphQL mutations in §9 — same
auth, same RBAC, same audit trail. No direct DB writes.

---

## 11. Observability

Every step emits structured `tracing` spans. Metrics exported to the
existing Prometheus `/metrics` endpoint:

- `crawl_candidates_total{source, outcome=seen|skipped|downloaded|handed_off|failed}` (counter)
- `crawl_http_requests_total{source, host, status}` (counter)
- `crawl_http_request_duration_seconds{source, host}` (histogram)
- `crawl_pdf_bytes_downloaded_total{source}` (counter)
- `crawl_dedup_hits_total{source, key=source_url|sha256}` (counter)
- `crawl_robots_disallowed_total{source, host}` (counter)
- `crawl_runs_active{source}` (gauge)

Log lines on any HTML-shape change (a selector that suddenly matches
zero elements when it used to match >1) — these are the early warning
that the source has reskinned and the parser needs an update.

---

## 12. Phasing

Each phase is shippable. No phase ships before the §0 gate is closed.

### Phase 0 — Gate
- [ ] Confirm robots.txt + ToS posture for rekhta.org.
- [ ] Define the initial public-domain allow-list of poets.
- [ ] Pick the takedown contact + SLA.

### Phase 1 — Scaffolding (no rekhta yet)
- [ ] Create `merk-crawl` crate with `Source`, `Crawler`, `Handoff`,
      `CrawlStore` traits.
- [ ] Migration: `crawl_source`, `crawl_run`, `crawl_target`.
- [ ] Polite HTTP client + robots cache + per-host rate limiter
      (unit-tested without network).
- [ ] A `Source` impl backed by a fixture filesystem — the test
      vector that proves the orchestrator + handoff work end-to-end
      against a local mock "site". This lands the handoff into the
      real `createIngestJob` flow.

### Phase 2 — Admin surface
- [ ] `admin/crawl.rs` GraphQL module + RBAC.
- [ ] Frontend admin "Sources" view (list + pause/resume + run
      start/cancel + target table).
- [ ] CLI binary (§10).

### Phase 3 — Rekhta source
- [ ] `sources/rekhta.rs` — listing parser, detail-page parser,
      PDF-URL extractor, skip-reason classifier.
- [ ] Allow-list-only: only candidates whose `author_name_raw`
      slug-matches a row in the §0 allow-list are resolved; everything
      else is skipped with reason `NotPublicDomain`.
- [ ] Replay-test fixtures (saved HTML snapshots) under
      `merk-crawl/tests/fixtures/rekhta/` so parser regressions are
      caught without hitting the network.

### Phase 4 — Second source (sanity check)
- [ ] `sources/archive_org.rs` against the Internet Archive's public
      API. Forces the `Source` abstraction to actually be one. If
      writing this requires changes to the trait, those changes go
      back into Phase 1 retroactively.

### Phase 5 — Production
- [ ] Scheduling (cron or systemd timer) for incremental re-crawls.
- [ ] Dashboard panels for the metrics in §11.
- [ ] Takedown drill — run the full path from `recordTakedown` to
      "book is unpublished and source is blacklisted" once, document
      the runbook.

---

## 13. Out of scope (explicitly)

These will be tempting; do not add them in the first cut.

- **Auto-publish.** The crawler never bypasses stages 3–5. A
  reviewer must approve every crawled book.
- **Image OCR by the crawler.** OCR is `merk-ingest` Stage 2 (currently
  deferred per `[[integration_plan_status]]` Item F). The crawler hands
  off the PDF and lets ingest do its job once Item F lands.
- **Multi-PDF books.** If rekhta serves a book as N volume-PDFs, the
  crawler stores them as N `crawl_target` rows that all reference the
  same `ingest_job`. Joining them into one logical book is a stage-3
  reviewer concern, not the crawler's.
- **De-DRM / unlock.** If a PDF is encrypted or watermarked beyond a
  trivial `pdf_password`, skip with reason `Drm`. No bypass.
- **Image scraping for covers.** Cover generation is `merk-ingest`
  Stage 3 (`covers.rs`). The crawler may *opportunistically* store a
  `cover_url` it found, but never downloads it — that's ingest's job.
- **User-supplied URLs.** Phase-1+ has no "paste a URL, crawl it"
  feature. Adding one expands the §0 surface enormously and needs its
  own threat model.

---

## 14. Open questions

1. **Does rekhta serve PDFs from `*.rekhta.org` or a CDN host?** If a
   CDN, that's a *second* host whose robots.txt + rate limit we honor.
2. **Hindi-script editions** — many Urdu classics on rekhta are
   available in Devanagari too. Same book? Same `crawl_target`?
   Separate `book` rows linked by `original_work`? This intersects
   with the catalog's `book.language` field — answer at Phase-3 review,
   not in the crawler.
3. **Audio recitations** — rekhta hosts MP3s of poetry recitations. Out
   of scope today; flag as a future `Source` for an entirely separate
   `audio` ingestion pipeline that doesn't exist yet.
4. **Pre-1923 / pre-1958 / pre-1971** — the right cutoff depends on
   jurisdiction (India: life+60, US: 1928 published works as of 2024).
   The §0 allow-list must encode *which* jurisdiction we're treating
   as authoritative.
