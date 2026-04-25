# REST API Design

## URL Conventions

- Slug-based routing — no raw SurrealDB IDs in URLs
- Number → slug redirect for chapters: `301 /books/:book/chapters/:n` → `/books/:book/chapters/:slug`
- Envelope: `{ "data": …, "meta": …, "links": { "self", "prev", "next" } }`
- Cursor pagination on list endpoints via `?cursor=<opaque>&limit=<n>`

---

## Endpoints

### Health

```
GET    /api/v1/health                     service health + DB latency
```

### Auth

```
POST   /api/v1/auth/register              register; returns JWT + user
POST   /api/v1/auth/login                 login; returns JWT + user
POST   /api/v1/auth/logout                stateless logout (auth)
POST   /api/v1/auth/forgot-password       request password reset email (no auth)
POST   /api/v1/auth/reset-password        reset password with emailed token (no auth)
GET    /api/v1/auth/me                    current user profile (auth)
PUT    /api/v1/auth/{id}/deactivate       deactivate account (auth)
```

### User / Profile (auth required)

```
GET    /api/v1/me                      current user + profile combined
PUT    /api/v1/me/profile              update profile fields
PUT    /api/v1/me/password             change password (requires old password)
DELETE /api/v1/me                      deactivate own account
GET    /api/v1/me/stats                aggregate reading counts
GET    /api/v1/me/reading-sessions     reading session history (?limit=&offset=)
GET    /api/v1/me/following            followed authors (?limit=&offset=)
```

### Books & Authors

```
GET    /api/v1/books                              list, filter, search (?q=&lang=&limit=&offset=)
POST   /api/v1/books                              create book (auth)
GET    /api/v1/books/:slug                        single book detail
PUT    /api/v1/books/:slug                        update book (auth)
GET    /api/v1/books/:slug/authors                authors of a book

GET    /api/v1/authors                            list + search (?q=)
POST   /api/v1/authors                            create author (auth)
GET    /api/v1/authors/:slug                      author detail + books
POST   /api/v1/authors/:slug/follow               follow (auth)
DELETE /api/v1/authors/:slug/follow               unfollow (auth)

GET    /api/v1/categories                         full tree
GET    /api/v1/categories/:slug/books             books in category
GET    /api/v1/tags/:slug/books                   books with tag
```

### Chapters

```
GET    /api/v1/books/:slug/chapters               table of contents
POST   /api/v1/books/:slug/chapters               create chapter (auth)
GET    /api/v1/books/:slug/chapters/:slug         chapter content + prev/next nav
GET    /api/v1/books/:slug/chapters/by-number/:n  resolve chapter by number → slug
```

### Reviews

```
GET    /api/v1/books/:slug/reviews                list book reviews (?spoilers=&limit=)
POST   /api/v1/books/:slug/reviews                submit book review (auth)
PUT    /api/v1/books/:slug/reviews/:id            update own review (auth)
POST   /api/v1/books/:slug/reviews/:id/vote       vote review helpful (auth)
POST   /api/v1/books/:slug/chapters/:slug/reviews submit chapter review (auth)
POST   /api/v1/reviews/:id/flag                   flag a review (auth)
```

### Highlights

```
GET    /api/v1/books/:slug/chapters/:slug/highlights  chapter highlights (?public=true)
POST   /api/v1/books/:slug/chapters/:slug/highlights  create highlight (auth)
GET    /api/v1/me/highlights                          my highlights across all books (auth)
PUT    /api/v1/highlights/:id                         update highlight (auth)
DELETE /api/v1/highlights/:id                         delete highlight (auth)
```

### Comments

```
GET    /api/v1/books/:slug/chapters/:slug/comments  list chapter comments
POST   /api/v1/books/:slug/chapters/:slug/comments  post comment (auth)
PUT    /api/v1/comments/:id                          edit own comment (auth)
DELETE /api/v1/comments/:id                          soft-delete comment (auth)
POST   /api/v1/comments/:id/vote                     vote comment +1/-1 (auth)
GET    /api/v1/highlights/:id/comments               comments on a highlight
```

### Translations (Word Glossary)

```
GET    /api/v1/books/:slug/chapters/:slug/translations  chapter-scoped lookup (?word=&lang=)
GET    /api/v1/books/:slug/translations                 book-scoped lookup (?word=&lang=)
GET    /api/v1/translations                             global lookup (?word=&lang=)
POST   /api/v1/translations                             submit translation (auth)
POST   /api/v1/translations/:id/vote                    vote +1/-1 (auth)
```

### Collections & Bookmarks

```
GET    /api/v1/me/collections                   my collections (auth)
POST   /api/v1/me/collections                   create collection (auth)
GET    /api/v1/me/collections/:id               single collection (auth)
PUT    /api/v1/me/collections/:id               update collection (auth)
DELETE /api/v1/me/collections/:id               delete collection (auth)
POST   /api/v1/me/collections/:id/books         add book to collection (auth)
DELETE /api/v1/me/collections/:id/books/:slug   remove book from collection (auth)

GET    /api/v1/me/bookmarks                     all bookmarks (?status=reading, auth)
PUT    /api/v1/books/:slug/bookmark             upsert bookmark (auth)
DELETE /api/v1/books/:slug/bookmark             remove bookmark (auth)

GET    /api/v1/me/reading-goal                  current year reading goal (auth)
PUT    /api/v1/me/reading-goal                  upsert reading goal (auth)
```

### Metrics

```
GET    /metrics                                  Prometheus metrics endpoint
```

---

## Query Parameters

| Param | Applies to | Example |
|---|---|---|
| `q` | books, authors | `?q=tolkien` |
| `lang` | books, translations | `?lang=en` |
| `status` | bookmarks | `?status=reading` |
| `sort` | reviews, books | `?sort=avg_rating&order=desc` |
| `filter[rating]` | reviews | `?filter[rating]=5` |
| `spoilers` | reviews, comments | `?spoilers=false` |
| `limit` / `offset` | all lists | `?limit=20&offset=0` |
| `include` | books | `?include=authors,categories` |
| `scope` | translations | `?scope=chapter&lang=en` |

---

## Chapter Response Shape

```json
{
  "data": {
    "id": "chapter:xyz",
    "number": 1,
    "title": "A Long-Expected Party",
    "slug": "a-long-expected-party",
    "summary": "Bilbo Baggins throws his eleventy-first birthday...",
    "content": "# A Long-Expected Party\n\nWhen Mr. Bilbo Baggins...",
    "content_format": "markdown",
    "word_count": 8420,
    "reading_time_mins": 36,
    "avg_rating": 4.7,
    "published_at": "2024-01-15T00:00:00Z",
    "book": { "title": "The Lord of the Rings", "slug": "lord-of-the-rings" },
    "navigation": {
      "prev": null,
      "next": { "number": 2, "slug": "the-shadow-of-the-past", "title": "The Shadow of the Past" }
    }
  },
  "meta": {
    "canonical_url": "https://example.com/books/lord-of-the-rings/chapters/a-long-expected-party",
    "description": "Read Chapter 1 of The Lord of the Rings by J.R.R. Tolkien.",
    "schema_org": {
      "@context": "https://schema.org",
      "@type": "Chapter",
      "name": "A Long-Expected Party",
      "position": 1,
      "isPartOf": { "@type": "Book", "name": "The Lord of the Rings" },
      "wordCount": 8420,
      "datePublished": "2024-01-15"
    }
  },
  "links": {
    "self": "/api/v1/books/lord-of-the-rings/chapters/a-long-expected-party",
    "book": "/api/v1/books/lord-of-the-rings",
    "next": "/api/v1/books/lord-of-the-rings/chapters/the-shadow-of-the-past"
  }
}
```

## HTTP Caching

```
Cache-Control: public, max-age=3600, stale-while-revalidate=86400
ETag: "{chapter_id}-{updated_at_unix}"
Last-Modified: {updated_at as RFC7231}
Vary: Accept-Encoding, Accept-Language
```
