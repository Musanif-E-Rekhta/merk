# GraphQL Examples

All GraphQL requests go to `POST /api/graphql`. Authenticated operations pass `Authorization: Bearer <token>` in the header.

---

## Auth & Profile

**Register**
```graphql
mutation Register {
  registerUser(username: "reader", email: "user@example.com", password: "OldPass1234!") {
    token
    user { id username email isActive isVerified }
  }
}
```

**Login**
```graphql
mutation Login {
  loginUser(email: "user@example.com", password: "OldPass1234!") {
    token
    user { id username email }
  }
}
```

**Request password reset email**
```graphql
mutation ForgotPassword {
  forgotPassword(email: "user@example.com")
}
```

**Reset password with token**
```graphql
mutation ResetPassword {
  resetPasswordWithToken(token: "a3f9...", newPassword: "NewPass5678!")
}
```

**Get current user with profile (authenticated)**
```graphql
query Me {
  me {
    id
    username
    email
    isActive
    isVerified
    profile {
      displayName
      bio
      language
      country
      avatarUrl
    }
  }
}
```

**Update profile (authenticated)**
```graphql
mutation UpdateProfile($input: UpdateProfileInput!) {
  updateProfile(input: $input) {
    id
    displayName
    bio
    language
    country
  }
}
```
```json
{
  "variables": {
    "input": { "displayName": "Strider", "bio": "Ranger of the North.", "language": "en", "country": "ME" }
  }
}
```

**Change password (authenticated)**
```graphql
mutation ChangePassword {
  changePassword(oldPassword: "OldPass1234!", newPassword: "NewPass5678!")
}
```

**Reading statistics and session history (authenticated)**
```graphql
query MyActivity($limit: Int) {
  myStats {
    booksReading
    booksCompleted
    highlightsCount
    reviewsCount
    readingSessionsCount
  }
  myReadingSessions(limit: $limit) {
    id
    bookId
    startedAt
    endedAt
    durationMins
    pageStart
    pageEnd
  }
  myFollowing(limit: $limit) {
    id
    name
    slug
    avatarUrl
  }
}
```
```json
{ "variables": { "limit": 10 } }
```
```json
{
  "data": {
    "myStats": {
      "booksReading": 3,
      "booksCompleted": 27,
      "highlightsCount": 184,
      "reviewsCount": 9,
      "readingSessionsCount": 63
    },
    "myReadingSessions": [
      { "id": "rs_001", "bookId": "book001", "startedAt": "2026-04-24T08:00:00Z", "durationMins": 42, "pageStart": 142, "pageEnd": 178 }
    ],
    "myFollowing": [
      { "id": "author_001", "name": "J.R.R. Tolkien", "slug": "j-r-r-tolkien", "avatarUrl": null }
    ]
  }
}
```

---

## Queries

**Fetch a chapter with navigation (canonical reader view)**
```graphql
query GetChapter($bookSlug: String!, $chapterSlug: String!) {
  chapter(bookSlug: $bookSlug, chapterSlug: $chapterSlug) {
    id
    number
    title
    slug
    content
    contentFormat
    summary
    wordCount
    readingTimeMins
    avgRating
    reviewCount
    prevChapter { number title slug }
    nextChapter { number title slug }
  }
}
```
```json
{
  "variables": {
    "bookSlug": "lord-of-the-rings",
    "chapterSlug": "a-long-expected-party"
  }
}
```
**Response:**
```json
{
  "data": {
    "chapter": {
      "id": "ch001",
      "number": 1,
      "title": "A Long-Expected Party",
      "slug": "a-long-expected-party",
      "content": "# A Long-Expected Party\n\nWhen Mr. Bilbo Baggins...",
      "contentFormat": "markdown",
      "summary": "Bilbo Baggins throws his eleventy-first birthday party.",
      "wordCount": 8420,
      "readingTimeMins": 36,
      "avgRating": 4.7,
      "reviewCount": 48,
      "prevChapter": null,
      "nextChapter": {
        "number": 2,
        "title": "The Shadow of the Past",
        "slug": "the-shadow-of-the-past"
      }
    }
  }
}
```

---

**List books with filter**
```graphql
query ListBooks($q: String, $lang: String, $limit: Int, $offset: Int) {
  books(q: $q, lang: $lang, limit: $limit, offset: $offset) {
    id
    title
    slug
    summary
    avgRating
    reviewCount
    chapterCount
    language
    isPublished
  }
}
```
```json
{ "variables": { "lang": "en", "q": "tolkien", "limit": 10, "offset": 0 } }
```

---

**List books by author**
```graphql
query BooksByAuthor($slug: String!, $limit: Int, $offset: Int) {
  booksByAuthor(slug: $slug, limit: $limit, offset: $offset) {
    id title slug avgRating isPublished
  }
}
```
```json
{ "variables": { "slug": "j-r-r-tolkien", "limit": 10, "offset": 0 } }
```

---

**Word translation lookup (priority: chapter → book → global)**
```graphql
query LookupWord($word: String!, $lang: String!, $bookSlug: String, $chapterSlug: String) {
  wordTranslations(
    word: $word
    targetLang: $lang
    bookSlug: $bookSlug
    chapterSlug: $chapterSlug
  ) {
    id
    word
    translation
    scope
    contextNote
    upvotes
    downvotes
    score
  }
}
```
```json
{
  "variables": {
    "word": "mellon",
    "lang": "en",
    "bookSlug": "lord-of-the-rings",
    "chapterSlug": "a-long-expected-party"
  }
}
```
**Response:**
```json
{
  "data": {
    "wordTranslations": [
      {
        "id": "wt_001",
        "word": "mellon",
        "translation": "friend (Sindarin Elvish)",
        "scope": "book",
        "contextNote": "The password to the Mines of Moria",
        "upvotes": 142,
        "downvotes": 3,
        "score": 139
      }
    ]
  }
}
```

---

**Get my bookmarks and reading goal (authenticated)**
```graphql
query MyShelf($year: Int) {
  myBookmarks(status: "reading", limit: 10, offset: 0) {
    id
    bookId
    status
    progress
    notes
  }
  myReadingGoal(year: $year) {
    year
    target
    completed
    progressPct
  }
}
```
*Header: `Authorization: Bearer eyJ...`*

```json
{
  "data": {
    "myBookmarks": [
      {
        "id": "bm_xyz",
        "bookId": "book001",
        "status": "reading",
        "progress": 142,
        "notes": "Started on holiday"
      }
    ],
    "myReadingGoal": {
      "year": 2026,
      "target": 24,
      "completed": 7,
      "progressPct": 29.17
    }
  }
}
```

---

**Chapter comments thread**
```graphql
query ChapterComments($bookSlug: String!, $chapterSlug: String!, $limit: Int) {
  chapterComments(bookSlug: $bookSlug, chapterSlug: $chapterSlug, limit: $limit) {
    id
    userId
    body
    isSpoiler
    isDeleted
    parentId
    offsetStart
    offsetEnd
    textSnapshot
  }
}
```

---

## Mutations

**Register and get a token**
```graphql
mutation Register {
  registerUser(
    username: "reader"
    email: "user@example.com"
    password: "OldPass1234!"
  ) {
    token
    user { id username email }
  }
}
```

---

**Upsert bookmark**
```graphql
mutation SetBookmark($input: UpsertBookmarkInput!) {
  upsertBookmark(input: $input) {
    id
    bookId
    status
    progress
  }
}
```
```json
{
  "variables": {
    "input": { "bookSlug": "lord-of-the-rings", "status": "reading", "progress": 142, "notes": "Holiday read" }
  }
}
```

---

**Submit a book review**
```graphql
mutation ReviewBook($input: CreateBookReviewInput!) {
  createBookReview(input: $input) {
    id
    rating
    title
    body
    verifiedReader
    helpfulCount
    status
  }
}
```
```json
{
  "variables": {
    "input": {
      "bookSlug": "lord-of-the-rings",
      "rating": 5,
      "title": "Timeless",
      "body": "Every re-read reveals something new.",
      "containsSpoiler": false,
      "readingStatus": "completed"
    }
  }
}
```

---

**Create a highlight**
```graphql
mutation Highlight($input: CreateHighlightInput!) {
  createHighlight(input: $input) {
    id
    textSnapshot
    color
    note
    isPublic
    offsetStart
    offsetEnd
  }
}
```
```json
{
  "variables": {
    "input": {
      "bookSlug": "lord-of-the-rings",
      "chapterSlug": "a-long-expected-party",
      "offsetStart": 1024,
      "offsetEnd": 1189,
      "paragraph": 3,
      "textSnapshot": "In a hole in the ground there lived a hobbit.",
      "color": "yellow",
      "note": "The iconic opening line",
      "isPublic": true
    }
  }
}
```

---

**Post a comment (inline, anchored to a passage)**
```graphql
mutation PostComment($input: CreateCommentInput!) {
  createComment(input: $input) {
    id
    body
    offsetStart
    offsetEnd
    textSnapshot
  }
}
```
```json
{
  "variables": {
    "input": {
      "bookSlug": "lord-of-the-rings",
      "chapterSlug": "a-long-expected-party",
      "body": "This line signals the turning point of the whole chapter.",
      "offsetStart": 3200,
      "offsetEnd": 3350,
      "textSnapshot": "...and he never came back.",
      "isSpoiler": false
    }
  }
}
```

---

**Submit a word translation**
```graphql
mutation SubmitTranslation($input: CreateTranslationInput!) {
  submitTranslation(input: $input) {
    id
    word
    translation
    scope
    upvotes
    score
  }
}
```
```json
{
  "variables": {
    "input": {
      "word": "lembas",
      "translation": "waybread — Elvish travel bread with extraordinary sustaining power",
      "sourceLang": "sjn",
      "targetLang": "en",
      "scope": "book",
      "bookSlug": "lord-of-the-rings",
      "contextNote": "First appears when the Company leaves Lothlórien"
    }
  }
}
```

---

**Set annual reading goal**
```graphql
mutation SetGoal($year: Int!, $target: Int!) {
  upsertReadingGoal(year: $year, target: $target) {
    year
    target
    completed
    progressPct
  }
}
```
```json
{ "variables": { "year": 2026, "target": 24 } }
```

---

**Create a collection and add a book**
```graphql
mutation CreateCollection($input: CreateCollectionInput!) {
  createCollection(input: $input) {
    id
    name
    isPublic
  }
}
```
```json
{
  "variables": {
    "input": { "name": "Epic Fantasy Essentials", "isPublic": true }
  }
}
```

```graphql
mutation AddToCollection($collectionId: ID!, $input: AddBookInput!) {
  addBookToCollection(collectionId: $collectionId, input: $input) {
    bookId
    position
    note
  }
}
```
```json
{
  "variables": {
    "collectionId": "col_001",
    "input": { "bookSlug": "lord-of-the-rings", "position": 1, "note": "Start here" }
  }
}
```
