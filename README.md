# merk

A blazing-fast Rust backend for a book reading platform, built on [Axum](https://github.com/tokio-rs/axum) + SurrealDB.

The service exposes a full REST API and a GraphQL API covering books, chapters, annotations (highlights, comments), community translations, reviews, collections, and bookmarks — all backed by SurrealDB graph relations and maintained by live event triggers.

## Features

- **REST & GraphQL Dual Support**: Every resource is accessible via slug-based REST routes (`/api/v1`) and a unified `async-graphql` schema at `/graphql`.
- **Auto-Generated OpenAPI Docs**: All REST routes are documented via `aide` and browsable in a Scalar UI with pre-configured JWT Bearer auth.
- **SurrealDB with Auto-Migrations**: Embedded `.surql` migration runner applies schema at startup — no external tool needed. Graph traversal powers RBAC, follows, and voting relations.
- **SurrealDB Event Triggers**: `book_review_stats`, `chapter_review_stats`, and `translation_vote_counts` events maintain denormalized `avg_rating`, `review_count`, `upvotes`, and `downvotes` fields without aggregation queries on every read.
- **Scoped Word Translations**: User-submitted word glossaries resolved in priority order: chapter → book → global. Community-voted via `+1 / -1` edges.
- **Full-Text Search**: SurrealDB SEARCH indexes on `book.title + summary` and `chapter.title + content + summary` with HIGHLIGHTS support.
- **RBAC via Graph Traversal**: `user --[assigned_role]--> role --[has_permission]--> permission` queried natively.
- **Infrastructure Monitoring**: Prometheus metrics at `/metrics`; OpenTelemetry traces and structured logs on every GraphQL operation.
- **Intelligent Configuration**: `envy` maps environment variables into strongly typed `AppConfig`, injected via Axum `State`.
- **Dynamic TLS Generation**: Toggleable HTTPS with an `rcgen` self-signed certificate on boot.
- **Granular Error Handling**: `thiserror` taxonomy; internal details never leak — `Internal` variants are logged server-side and surface as opaque `500` responses.

## Getting Started

### Prerequisites

- [Rust Toolchain (1.70+)](https://rustup.rs/)
- [Docker Engine & Docker Compose](https://docs.docker.com/engine/install/)

### Spin up Local Dependencies

```bash
docker-compose up -d
```

This launches:
- SurrealDB on port `8000` (in-memory mode)
- Prometheus on port `9090` (scrapes `:9678/metrics` every 5 s)

### Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `127.0.0.1` | Network interface to bind to |
| `PORT` | `9678` / `8443` | Listen port (`9678` HTTP, `8443` HTTPS) |
| `ENABLE_TLS` | `false` | Toggle auto-generated self-signed TLS via `rcgen` |
| `TLS_ALT_NAME` | *(empty)* | Extra SAN in the generated cert |
| `SURREALDB_URL` | `ws://127.0.0.1:8000` | SurrealDB connection URL |
| `SURREALDB_USER` | `root` | SurrealDB root username |
| `SURREALDB_PASS` | `root` | SurrealDB root password |
| `SURREALDB_NS` | `merk` | SurrealDB namespace |
| `SURREALDB_DB` | `merk` | SurrealDB database name |
| `JWT_SECRET` | *(dev default)* | HS256 signing secret — **must be ≥ 32 chars**; dev default rejected in `--release` |

Copy `.env.local` and adjust as needed, then:

```bash
cargo run
```

The terminal displays an ASCII banner with the active bind address and TLS state.

## REST API — `/api/v1`

All authenticated endpoints require `Authorization: Bearer <token>`.

### Auth

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/health` | — | Service health + DB latency |
| `POST` | `/api/v1/auth/register` | — | Register; returns JWT + user |
| `POST` | `/api/v1/auth/login` | — | Login; returns JWT + user |
| `POST` | `/api/v1/auth/logout` | Bearer | Stateless logout (204) |
| `POST` | `/api/v1/auth/forgot-password` | — | Request password reset email |
| `POST` | `/api/v1/auth/reset-password` | — | Reset password with emailed token |
| `GET` | `/api/v1/auth/me` | Bearer | Current user profile |
| `PUT` | `/api/v1/auth/{id}/deactivate` | Bearer | Deactivate account |

### User / Profile

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/me` | Bearer | Current user + profile combined |
| `PUT` | `/api/v1/me/profile` | Bearer | Update profile fields |
| `PUT` | `/api/v1/me/password` | Bearer | Change password (old + new) |
| `DELETE` | `/api/v1/me` | Bearer | Deactivate own account |
| `GET` | `/api/v1/me/stats` | Bearer | Aggregate reading counts |
| `GET` | `/api/v1/me/reading-sessions` | Bearer | Reading session history (`?limit=&offset=`) |
| `GET` | `/api/v1/me/following` | Bearer | Followed authors (`?limit=&offset=`) |

### Books

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/books` | — | List/search books (`?q=&lang=&limit=&offset=`) |
| `POST` | `/api/v1/books` | Bearer | Create a book |
| `GET` | `/api/v1/books/:slug` | — | Book detail |
| `PUT` | `/api/v1/books/:slug` | Bearer | Update book |
| `GET` | `/api/v1/books/:slug/authors` | — | Authors of a book |
| `GET` | `/api/v1/authors` | — | List/search authors |
| `POST` | `/api/v1/authors` | Bearer | Create author |
| `GET` | `/api/v1/authors/:slug` | — | Author detail |
| `POST` | `/api/v1/authors/:slug/follow` | Bearer | Follow author |
| `DELETE` | `/api/v1/authors/:slug/follow` | Bearer | Unfollow author |
| `GET` | `/api/v1/categories` | — | Category tree |
| `GET` | `/api/v1/categories/:slug/books` | — | Books in category |
| `GET` | `/api/v1/tags/:slug/books` | — | Books with tag |

### Chapters

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/books/:slug/chapters` | — | Table of contents |
| `POST` | `/api/v1/books/:slug/chapters` | Bearer | Create chapter |
| `GET` | `/api/v1/books/:slug/chapters/:slug` | — | Chapter content + nav |
| `GET` | `/api/v1/books/:slug/chapters/by-number/:n` | — | Resolve chapter by number |

### Reviews

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/books/:slug/reviews` | — | Book reviews (`?spoilers=&limit=`) |
| `POST` | `/api/v1/books/:slug/reviews` | Bearer | Submit book review |
| `PUT` | `/api/v1/books/:slug/reviews/:id` | Bearer | Update own review |
| `POST` | `/api/v1/books/:slug/reviews/:id/vote` | Bearer | Vote review helpful |
| `POST` | `/api/v1/books/:slug/chapters/:slug/reviews` | Bearer | Submit chapter review |
| `POST` | `/api/v1/reviews/:id/flag` | Bearer | Flag a review |

### Highlights

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/books/:slug/chapters/:slug/highlights` | — | Chapter highlights (`?public=true`) |
| `POST` | `/api/v1/books/:slug/chapters/:slug/highlights` | Bearer | Create highlight |
| `GET` | `/api/v1/me/highlights` | Bearer | My highlights across all books |
| `PUT` | `/api/v1/highlights/:id` | Bearer | Update highlight |
| `DELETE` | `/api/v1/highlights/:id` | Bearer | Delete highlight |

### Comments

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/books/:slug/chapters/:slug/comments` | — | Chapter comments |
| `POST` | `/api/v1/books/:slug/chapters/:slug/comments` | Bearer | Post comment |
| `PUT` | `/api/v1/comments/:id` | Bearer | Edit own comment |
| `DELETE` | `/api/v1/comments/:id` | Bearer | Soft-delete comment |
| `POST` | `/api/v1/comments/:id/vote` | Bearer | Vote comment (`+1` / `-1`) |
| `GET` | `/api/v1/highlights/:id/comments` | — | Comments on a highlight |

### Translations (Word Glossary)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/books/:slug/chapters/:slug/translations` | — | Lookup word (`?word=&lang=`), chapter-scoped |
| `GET` | `/api/v1/books/:slug/translations` | — | Lookup word, book-scoped |
| `GET` | `/api/v1/translations` | — | Global word lookup |
| `POST` | `/api/v1/translations` | Bearer | Submit translation |
| `POST` | `/api/v1/translations/:id/vote` | Bearer | Vote translation (`+1` / `-1`) |

### Collections & Bookmarks

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/me/collections` | Bearer | My collections |
| `POST` | `/api/v1/me/collections` | Bearer | Create collection |
| `GET` | `/api/v1/me/collections/:id` | Bearer | Single collection |
| `PUT` | `/api/v1/me/collections/:id` | Bearer | Update collection |
| `DELETE` | `/api/v1/me/collections/:id` | Bearer | Delete collection |
| `POST` | `/api/v1/me/collections/:id/books` | Bearer | Add book to collection |
| `DELETE` | `/api/v1/me/collections/:id/books/:slug` | Bearer | Remove book from collection |
| `GET` | `/api/v1/me/bookmarks` | Bearer | My bookmarks (`?status=reading`) |
| `PUT` | `/api/v1/books/:slug/bookmark` | Bearer | Upsert bookmark |
| `DELETE` | `/api/v1/books/:slug/bookmark` | Bearer | Remove bookmark |
| `GET` | `/api/v1/me/reading-goal` | Bearer | Current year reading goal |
| `PUT` | `/api/v1/me/reading-goal` | Bearer | Upsert reading goal |

### Metrics

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/metrics` | Prometheus metrics endpoint |

## GraphQL — `/graphql`

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/graphql` | GraphiQL browser UI |
| `POST` | `/graphql` | GraphQL executor |

Pass `Authorization: Bearer <token>` in the request header for authenticated operations.

### Queries

| Operation | Description |
|-----------|-------------|
| `me` | Current authenticated user with profile |
| `myStats` | Aggregate reading counts (auth) |
| `myReadingSessions(limit, offset)` | Reading session history (auth) |
| `myFollowing(limit, offset)` | Followed authors (auth) |
| `books(q, lang, limit, offset)` | Search/list books |
| `book(slug)` | Single book detail |
| `booksByAuthor(slug, limit, offset)` | Books by a given author |
| `booksByCategory(slug, limit, offset)` | Books in a category |
| `booksByTag(slug, limit, offset)` | Books with a tag |
| `authors(q, limit, offset)` | Search/list authors |
| `author(slug)` | Author detail |
| `categories` | Full category tree |
| `chapters(bookSlug, limit, offset)` | Table of contents |
| `chapter(bookSlug, chapterSlug)` | Chapter with nav, content, stats |
| `bookReviews(bookSlug, limit, offset)` | Book reviews |
| `chapterReviews(bookSlug, chapterSlug, limit, offset)` | Chapter reviews |
| `chapterHighlights(bookSlug, chapterSlug, publicOnly, limit, offset)` | Public/own highlights |
| `myHighlights(limit, offset)` | My highlights across books |
| `chapterComments(bookSlug, chapterSlug, limit, offset)` | Threaded comments |
| `highlightComments(highlightId, limit, offset)` | Comments on a highlight |
| `commentReplies(parentId)` | Replies to a comment |
| `wordTranslations(word, targetLang, bookSlug, chapterSlug)` | Priority-ordered translations |
| `myCollections(limit, offset)` | My collections |
| `collection(id)` | Single collection |
| `collectionBooks(collectionId, limit, offset)` | Books in a collection |
| `myBookmarks(status, limit, offset)` | My bookmarks |
| `myReadingGoal(year)` | Reading goal for a year |

### Mutations

| Operation | Description |
|-----------|-------------|
| `registerUser / loginUser / logoutUser` | Auth |
| `forgotPassword / resetPasswordWithToken` | Password reset (unauthenticated) |
| `updateProfile / changePassword / deleteMe` | Profile management (auth) |
| `createBook / updateBook` | Books (auth) |
| `createAuthor / followAuthor / unfollowAuthor` | Authors (auth) |
| `createChapter / updateChapter` | Chapters (auth) |
| `createBookReview / updateBookReview / deleteBookReview / voteBookReview` | Book reviews (auth) |
| `createChapterReview / voteChapterReview / flagReview` | Chapter reviews + flagging (auth) |
| `createHighlight / updateHighlight / deleteHighlight` | Highlights (auth) |
| `createComment / updateComment / deleteComment / voteComment` | Comments (auth) |
| `submitTranslation / voteTranslation` | Word translations (auth) |
| `createCollection / updateCollection / deleteCollection` | Collections (auth) |
| `addBookToCollection / removeBookFromCollection` | Collection membership (auth) |
| `upsertBookmark / removeBookmark` | Reading shelf (auth) |
| `upsertReadingGoal` | Annual reading goal (auth) |

### Interactive Explorers

| Tool | URL |
|------|-----|
| OpenAPI / Scalar | `http://127.0.0.1:9678/docs/scalar` |
| OpenAPI JSON | `http://127.0.0.1:9678/docs/openapi.json` |
| GraphQL / GraphiQL | `http://127.0.0.1:9678/graphql` |

*(Prefix with `https://` when `ENABLE_TLS=true`.)*

## Database Schema

Migrations are embedded in the binary via `rust-embed` and applied automatically on startup. The `_migrations` table tracks what has been applied.

### Migration 0001 — Auth & Users

| Table | Key Fields |
|-------|-----------|
| `user` | `username` (unique), `email` (unique), `password_hash`, `is_active`, `is_verified` |
| `profile` | `user` (link), `display_name`, `avatar_url`, `bio`, `language`, `country`, `timezone` |

### Migration 0002 — RBAC Graphs

```
user --[assigned_role]--> role --[has_permission]--> permission
```

Seeded roles: `admin`, `user`. Seeded permissions: `manage_users`, `read_content`, `write_content`.

### Migration 0003 — Book Platform (24 tables)

| Group | Tables |
|-------|--------|
| **Content** | `author`, `publisher`, `category`, `tag`, `book`, `chapter` |
| **Graph Relations** | `wrote` (author→book), `follows` (user→author), `bookmark` (user→book), `collection_book` (collection→book) |
| **Reading** | `reading_session`, `reading_goal`, `collection` |
| **Annotations** | `highlight`, `comment`, `comment_vote` |
| **Reviews** | `book_review`, `chapter_review`, `book_review_vote`, `chapter_review_vote`, `review_flag` |
| **Translations** | `word_translation`, `translation_vote` |

Key design decisions:

| Decision | Reason |
|----------|--------|
| `wrote`, `bookmark`, `follows` as RELATION tables | Native SurrealDB graph traversal: `SELECT ->wrote->book FROM author:x` |
| Tags/categories embedded as arrays on `book` | Fast display reads; graph traversal only when filtering |
| `chapter.slug` unique per book, not globally | `/books/a/chapters/prologue` and `/books/b/chapters/prologue` are both valid |
| `avg_rating` / `review_count` denormalized on `book` and `chapter` | Maintained by SurrealDB events; avoids aggregation on every page load |
| `text_snapshot` on highlight and comment | Character offsets break on content edits; snapshot preserves what the user selected |
| `comment.deleted_at` soft delete | Deleting a parent would orphan the reply thread |
| `verified_reader` written at review creation time | Avoids a bookmark join on every review read |

**SurrealDB Event Triggers:**

| Event | Trigger | Effect |
|-------|---------|--------|
| `book_review_stats` | `book_review` CREATE/UPDATE/DELETE | Recalculates `book.avg_rating`, `book.review_count` |
| `chapter_review_stats` | `chapter_review` CREATE/UPDATE/DELETE | Recalculates `chapter.avg_rating`, `chapter.review_count` |
| `book_review_helpful_count` | `book_review_vote` CREATE/DELETE | Syncs `book_review.helpful_count` |
| `translation_vote_counts` | `translation_vote` CREATE/DELETE | Syncs `word_translation.upvotes`, `.downvotes` |

See [`docs/schema_design.md`](docs/schema_design.md) for the index, or jump directly to:
[entities](docs/schema/01_entities.md) · [events & decisions](docs/schema/02_events_decisions.md) · [ERD](docs/schema/03_erd.md) · [REST API](docs/schema/04_rest_api.md) · [GraphQL schema](docs/schema/05_graphql.md) · [HTTP examples](docs/schema/06_http_examples.md) · [GraphQL examples](docs/schema/07_graphql_examples.md)

## Authentication

- Tokens are **HS256 JWTs** signed with `JWT_SECRET`, valid for **24 hours**.
- Pass as `Authorization: Bearer <token>`.
- Logout is client-side only — the server does not maintain a token blocklist.
- Suspended users (`is_active = false`) receive `403 Forbidden` on login and on every authenticated request.

## Testing

Tests run against an in-memory SurrealDB instance — no external services required:

```bash
cargo test
```

A Criterion benchmark for the Argon2 hashing layer:

```bash
cargo bench
```

## Deployment

### Docker

```bash
docker-compose up --build
```

### Kubernetes (Timoni)

The `timoni/` directory contains a [Timoni](https://timoni.sh/) bundle (CUE-based):

- `Deployment` for `merk` with liveness/readiness probes on `/api/v1/health`
- Co-deployed in-memory SurrealDB `Deployment` + `Service`
- `ConfigMap` for non-secret env vars; `Secret` for `JWT_SECRET` and `SURREALDB_PASS`

Default image: `ghcr.io/musanif-e-rekhta/merk:latest`.  
Resource defaults: 100 m CPU / 128 Mi RAM; limits 500 m / 512 Mi.

### CI

GitHub Actions (`.github/workflows/ci.yml`) runs on every push:
1. `cargo fmt --check`
2. `cargo clippy`
3. `cargo test --all-features`
4. `cargo build --release` (artifact upload)

## Architecture

```
src/
  main.rs             — binary entry point, telemetry init, env loading
  lib.rs              — re-exports all public modules
  server.rs           — server startup, TLS, Prometheus, graceful shutdown
  config.rs           — AppConfig (envy env → typed struct)
  state.rs            — AppState (Arc<AppConfig> + Db, injected via Axum State)
  error.rs            — thiserror taxonomy, IntoResponse, ErrorResponse, OperationOutput
  api/
    mod.rs            — create_router(): merges sub-routers, TraceLayer, CORS
    v1/
      health.rs       — GET /health
      users.rs        — auth REST handlers
      books.rs        — books, authors, categories, tags
      chapters.rs     — chapter content + table of contents
      reviews.rs      — book + chapter reviews, flagging, voting
      highlights.rs   — chapter highlights + personal feed
      comments.rs     — threaded comments, inline anchoring, voting
      translations.rs — word glossary (chapter/book/global scoped)
      collections.rs  — collections, bookmarks, reading goals
    graphql/          — async-graphql schema + axum integration
      mod.rs          — merged QueryRoot + MutationRoot, JWT extraction
      users.rs        — UserQuery + UserMutation
      books.rs        — BookQuery + BookMutation
      chapters.rs     — ChapterQuery + ChapterMutation
      reviews.rs      — ReviewQuery + ReviewMutation
      highlights.rs   — HighlightQuery + HighlightMutation
      comments.rs     — CommentQuery + CommentMutation
      translations.rs — TranslationQuery + TranslationMutation
      collections.rs  — CollectionQuery + CollectionMutation (+ bookmarks, goals)
    openapi/          — aide OpenAPI spec generation, Scalar UI
    middleware/       — Claims extractor (JWT Bearer validation)
  db/
    mod.rs            — Db type alias, connect_to_db(), migration runner
    user_repo.rs      — UserRepo CRUD + auth ops
    profile_repo.rs   — ProfileRepo
    rbac_repo.rs      — RbacRepo (graph-based permission checks)
    book_repo.rs      — BookRepo, AuthorRepo, CategoryRepo, TagRepo
    chapter_repo.rs   — ChapterRepo (content, nav, TOC)
    review_repo.rs    — BookReviewRepo, ChapterReviewRepo, ReviewFlagRepo
    highlight_repo.rs — HighlightRepo
    comment_repo.rs   — CommentRepo (threaded, voting)
    translation_repo.rs — TranslationRepo (scoped lookup + voting)
    collection_repo.rs  — CollectionRepo, CollectionBookRepo
    bookmark_repo.rs    — BookmarkRepo, ReadingGoalRepo
    migrations/       — .surql migration files embedded at compile time
  services/
    auth.rs           — hash_password(), verify_password(), generate_jwt()
```

**Key design decisions:**
- `AppState` clones cheaply — `AppConfig` is behind `Arc`, `Db` is an internal `Arc`-backed handle.
- Errors never leak internal details; `Internal` and `Upstream` variants log server-side and surface as `500`/`502` with opaque messages.
- All DB integration tests use `mem://` SurrealDB so CI never needs a running database.
- aide's `axum-query` feature is required for `Query<T>` extractors to implement `OperationInput`.

## License

CC0 1.0 Universal. See `LICENSE`.
