# HTTP Examples

All requests go to `https://<host>/api/v1`. Authenticated endpoints require `Authorization: Bearer <token>`.

---

## Auth & Profile

**Register**
```http
POST /api/v1/auth/register
Content-Type: application/json

{ "username": "aragorn", "email": "aragorn@gondor.me", "password": "Isildur1234!" }
```
```json
HTTP/1.1 201 Created

{ "token": "eyJ...", "user": { "id": "user_abc", "username": "aragorn", "email": "aragorn@gondor.me" } }
```

**Login**
```http
POST /api/v1/auth/login
Content-Type: application/json

{ "email": "aragorn@gondor.me", "password": "Isildur1234!" }
```

**Request password reset email** *(no auth — always returns 200 to avoid leaking email existence)*
```http
POST /api/v1/auth/forgot-password
Content-Type: application/json

{ "email": "aragorn@gondor.me" }
```

**Reset password with emailed token**
```http
POST /api/v1/auth/reset-password
Content-Type: application/json

{ "token": "a3f9...", "new_password": "NewPass5678!" }
```

**Get current user + profile** *(auth required)*
```http
GET /api/v1/me
Authorization: Bearer eyJ...
```
```json
{
  "id": "user_abc",
  "username": "aragorn",
  "email": "aragorn@gondor.me",
  "is_active": true,
  "is_verified": true,
  "profile": {
    "display_name": "Aragorn",
    "bio": "Ranger of the North.",
    "language": "en",
    "country": "ME"
  }
}
```

**Update profile** *(auth required)*
```http
PUT /api/v1/me/profile
Authorization: Bearer eyJ...
Content-Type: application/json

{ "display_name": "Strider", "bio": "Ranger of the North.", "language": "en", "country": "ME" }
```

**Change password** *(auth required)*
```http
PUT /api/v1/me/password
Authorization: Bearer eyJ...
Content-Type: application/json

{ "old_password": "Isildur1234!", "new_password": "NewPass5678!" }
```
```
HTTP/1.1 204 No Content
```

**Get reading statistics** *(auth required)*
```http
GET /api/v1/me/stats
Authorization: Bearer eyJ...
```
```json
{
  "books_reading": 3,
  "books_completed": 27,
  "books_read_later": 12,
  "books_dropped": 2,
  "highlights_count": 184,
  "reviews_count": 9,
  "reading_sessions_count": 63
}
```

---

## Books

**List books with search and language filter**
```http
GET /api/v1/books?q=tolkien&lang=en&limit=10&offset=0
```
```json
[
  {
    "id": "0r9tg2kxyz",
    "title": "The Lord of the Rings",
    "slug": "lord-of-the-rings",
    "summary": "One Ring to rule them all...",
    "language": "en",
    "avg_rating": 4.8,
    "review_count": 2341,
    "chapter_count": 62,
    "is_published": true,
    "cover_url": "https://cdn.example.com/lotr.jpg",
    "created_at": "2024-01-10T00:00:00Z",
    "updated_at": "2024-06-01T12:00:00Z"
  }
]
```

**Get a single book**
```http
GET /api/v1/books/lord-of-the-rings
```

**Create a book** *(auth required)*
```http
POST /api/v1/books
Authorization: Bearer eyJ...
Content-Type: application/json

{
  "title": "The Silmarillion",
  "slug": "the-silmarillion",
  "summary": "The mythology and early history of Middle-earth.",
  "language": "en",
  "isbn": "978-0-261-10237-9"
}
```
```json
HTTP/1.1 201 Created

{
  "id": "1a2b3c4d5e",
  "title": "The Silmarillion",
  "slug": "the-silmarillion",
  "avg_rating": null,
  "review_count": 0,
  "chapter_count": 0,
  "is_published": false
}
```

---

## Chapters

**Table of contents for a book**
```http
GET /api/v1/books/lord-of-the-rings/chapters
```
```json
[
  { "id": "ch001", "number": 1, "title": "A Long-Expected Party", "slug": "a-long-expected-party", "reading_time_mins": 36, "avg_rating": 4.7, "is_published": true },
  { "id": "ch002", "number": 2, "title": "The Shadow of the Past", "slug": "the-shadow-of-the-past", "reading_time_mins": 42, "avg_rating": 4.6, "is_published": true }
]
```

**Fetch a chapter by slug** *(scraper-friendly canonical URL)*
```http
GET /api/v1/books/lord-of-the-rings/chapters/a-long-expected-party
```
```json
{
  "id": "ch001",
  "book_id": "lotr001",
  "number": 1,
  "title": "A Long-Expected Party",
  "slug": "a-long-expected-party",
  "content": "# A Long-Expected Party\n\nWhen Mr. Bilbo Baggins...",
  "content_format": "markdown",
  "summary": "Bilbo Baggins throws his eleventy-first birthday party and vanishes.",
  "meta_description": "Read Chapter 1 of The Lord of the Rings — A Long-Expected Party by J.R.R. Tolkien.",
  "word_count": 8420,
  "reading_time_mins": 36,
  "avg_rating": 4.7,
  "review_count": 48,
  "is_published": true,
  "published_at": "2024-01-15T00:00:00Z",
  "updated_at": "2024-06-01T12:00:00Z",
  "prev_chapter": null,
  "next_chapter": {
    "number": 2,
    "title": "The Shadow of the Past",
    "slug": "the-shadow-of-the-past"
  }
}
```

**Redirect: chapter by number → slug**
```http
GET /api/v1/books/lord-of-the-rings/chapters/by-number/1
```
```json
HTTP/1.1 200 OK

{ "slug": "a-long-expected-party" }
```
*(Client issues a 301 redirect to the canonical slug URL)*

---

## Bookmark

**Mark a book as "reading"** *(auth required)*
```http
PUT /api/v1/books/lord-of-the-rings/bookmark
Authorization: Bearer eyJ...
Content-Type: application/json

{
  "status": "reading",
  "progress": 142,
  "notes": "Started on holiday"
}
```
```json
{
  "id": "bm_xyz",
  "book_id": "lotr001",
  "status": "reading",
  "progress": 142,
  "notes": "Started on holiday",
  "started_at": "2026-04-24T08:30:00Z",
  "completed_at": null,
  "updated_at": "2026-04-24T08:30:00Z"
}
```

**List my bookmarks filtered by status** *(auth required)*
```http
GET /api/v1/me/bookmarks?status=reading&limit=20&offset=0
Authorization: Bearer eyJ...
```

**Remove a bookmark** *(auth required)*
```http
DELETE /api/v1/books/lord-of-the-rings/bookmark
Authorization: Bearer eyJ...
```
```
HTTP/1.1 204 No Content
```

---

## Reviews

**List book reviews (no spoilers)**
```http
GET /api/v1/books/lord-of-the-rings/reviews?spoilers=false&limit=10&offset=0
```
```json
[
  {
    "id": "rev_001",
    "user_id": "user_abc",
    "book_id": "lotr001",
    "rating": 5,
    "title": "A timeless masterpiece",
    "body": "Tolkien's world-building is unparalleled...",
    "contains_spoiler": false,
    "reading_status": "completed",
    "verified_reader": true,
    "helpful_count": 312,
    "status": "published",
    "created_at": "2024-03-10T00:00:00Z",
    "updated_at": "2024-03-10T00:00:00Z"
  }
]
```

**Submit a book review** *(auth required)*
```http
POST /api/v1/books/lord-of-the-rings/reviews
Authorization: Bearer eyJ...
Content-Type: application/json

{
  "rating": 5,
  "title": "Changed my life",
  "body": "I read this every year without fail.",
  "contains_spoiler": false,
  "reading_status": "completed"
}
```

**Vote a review as helpful** *(auth required)*
```http
POST /api/v1/books/lord-of-the-rings/reviews/rev_001/vote
Authorization: Bearer eyJ...
Content-Type: application/json

{ "value": 1 }
```

**Submit a chapter review** *(auth required)*
```http
POST /api/v1/books/lord-of-the-rings/chapters/a-long-expected-party/reviews
Authorization: Bearer eyJ...
Content-Type: application/json

{ "rating": 5, "body": "Perfect opening chapter.", "contains_spoiler": false }
```

**Flag a review** *(auth required)*
```http
POST /api/v1/reviews/rev_001/flag
Authorization: Bearer eyJ...
Content-Type: application/json

{ "reason": "spoiler", "note": "Reveals the ending without warning" }
```

---

## Highlights

**Create a highlight** *(auth required)*
```http
POST /api/v1/books/lord-of-the-rings/chapters/a-long-expected-party/highlights
Authorization: Bearer eyJ...
Content-Type: application/json

{
  "offset_start": 1024,
  "offset_end": 1189,
  "paragraph": 3,
  "text_snapshot": "In a hole in the ground there lived a hobbit.",
  "color": "yellow",
  "note": "The iconic opening line",
  "is_public": true
}
```
```json
HTTP/1.1 201 Created

{
  "id": "hl_abc",
  "user_id": "user_abc",
  "chapter_id": "ch001",
  "book_id": "lotr001",
  "offset_start": 1024,
  "offset_end": 1189,
  "paragraph": 3,
  "text_snapshot": "In a hole in the ground there lived a hobbit.",
  "color": "yellow",
  "note": "The iconic opening line",
  "is_public": true,
  "created_at": "2026-04-24T09:00:00Z"
}
```

**List public highlights for a chapter**
```http
GET /api/v1/books/lord-of-the-rings/chapters/a-long-expected-party/highlights?public=true
```

---

## Comments

**Post a chapter comment** *(auth required)*
```http
POST /api/v1/books/lord-of-the-rings/chapters/a-long-expected-party/comments
Authorization: Bearer eyJ...
Content-Type: application/json

{
  "body": "The pacing in this chapter is masterful — slow burn that hooks you completely.",
  "is_spoiler": false
}
```

**Reply to a comment** *(auth required)*
```http
POST /api/v1/books/lord-of-the-rings/chapters/a-long-expected-party/comments
Authorization: Bearer eyJ...
Content-Type: application/json

{
  "body": "Agreed — Tolkien takes his time but every word counts.",
  "parent_id": "cm_xyz",
  "is_spoiler": false
}
```

**Inline comment anchored to a passage** *(auth required)*
```http
POST /api/v1/books/lord-of-the-rings/chapters/a-long-expected-party/comments
Authorization: Bearer eyJ...
Content-Type: application/json

{
  "body": "This is exactly where the adventure truly begins.",
  "offset_start": 3200,
  "offset_end": 3350,
  "text_snapshot": "...and he never came back.",
  "is_spoiler": false
}
```

**Vote a comment** *(auth required)*
```http
POST /api/v1/comments/cm_xyz/vote
Authorization: Bearer eyJ...
Content-Type: application/json

{ "value": 1 }
```

---

## Translations (Word Glossary)

**Look up a word — chapter-scoped, falls back to book, then global**
```http
GET /api/v1/books/lord-of-the-rings/chapters/a-long-expected-party/translations?word=mellon&lang=en
```
```json
[
  {
    "id": "wt_001",
    "word": "mellon",
    "translation": "friend (Sindarin Elvish)",
    "source_lang": "sjn",
    "target_lang": "en",
    "scope": "book",
    "book_id": "lotr001",
    "chapter_id": null,
    "context_note": "The password to the Mines of Moria",
    "upvotes": 142,
    "downvotes": 3,
    "score": 139,
    "submitted_by": "user_abc",
    "created_at": "2024-02-01T00:00:00Z"
  }
]
```

**Submit a word translation** *(auth required)*
```http
POST /api/v1/translations
Authorization: Bearer eyJ...
Content-Type: application/json

{
  "word": "lembas",
  "translation": "waybread — Elvish travel bread, sustains the eater far beyond ordinary food",
  "source_lang": "sjn",
  "target_lang": "en",
  "scope": "book",
  "book_slug": "lord-of-the-rings",
  "context_note": "First appears in The Fellowship of the Ring"
}
```

**Vote on a translation** *(auth required)*
```http
POST /api/v1/translations/wt_001/vote
Authorization: Bearer eyJ...
Content-Type: application/json

{ "value": 1 }
```

---

## Collections

**Create a collection** *(auth required)*
```http
POST /api/v1/me/collections
Authorization: Bearer eyJ...
Content-Type: application/json

{ "name": "Epic Fantasy Essentials", "description": "My curated list of must-reads", "is_public": true }
```

**Add a book to a collection** *(auth required)*
```http
POST /api/v1/me/collections/col_001/books
Authorization: Bearer eyJ...
Content-Type: application/json

{ "book_slug": "lord-of-the-rings", "position": 1, "note": "Start here" }
```

**Remove a book from a collection** *(auth required)*
```http
DELETE /api/v1/me/collections/col_001/books/lord-of-the-rings
Authorization: Bearer eyJ...
```

---

## Reading Goal

**Set a yearly reading goal** *(auth required)*
```http
PUT /api/v1/me/reading-goal
Authorization: Bearer eyJ...
Content-Type: application/json

{ "year": 2026, "target": 24 }
```
```json
{
  "id": "rg_001",
  "year": 2026,
  "target": 24,
  "completed": 7,
  "progress_pct": 29.17
}
```
