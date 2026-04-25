# Book Reading Platform — Schema Design

> This document is an index. Each section lives in its own file under [`docs/schema/`](schema/).

---

## Contents

| # | File | What's inside |
|---|------|---------------|
| 1 | [01_entities.md](schema/01_entities.md) | Entity overview, all 25 table definitions (fields, types, indexes) |
| 2 | [02_events_decisions.md](schema/02_events_decisions.md) | SurrealDB event triggers + key design decisions and their rationale |
| 3 | [03_erd.md](schema/03_erd.md) | Mermaid entity-relationship diagram covering all 25 tables |
| 4 | [04_rest_api.md](schema/04_rest_api.md) | REST endpoints, URL conventions, query params, chapter response shape, HTTP caching headers |
| 5 | [05_graphql.md](schema/05_graphql.md) | Full `async-graphql` schema — types, queries, mutations, input types |
| 6 | [06_http_examples.md](schema/06_http_examples.md) | Runnable HTTP request/response examples for every resource |
| 7 | [07_graphql_examples.md](schema/07_graphql_examples.md) | GraphQL query and mutation examples with variables and responses |

---

## Entity Groups at a Glance

| Group | Tables |
|---|---|
| **Identity** | `user`, `profile` *(migration 0001)* |
| **Content** | `author`, `publisher`, `category`, `tag`, `book`, `chapter` |
| **Graph Relations** | `wrote`, `follows`, `bookmark`, `collection_book` |
| **Reading** | `reading_session`, `reading_goal`, `collection` |
| **Annotations** | `highlight`, `comment`, `comment_vote` |
| **Reviews** | `book_review`, `chapter_review`, `book_review_vote`, `chapter_review_vote`, `review_flag` |
| **Translations** | `word_translation`, `translation_vote` |

**25 tables** across 7 groups — all defined in `src/db/migrations/`.
