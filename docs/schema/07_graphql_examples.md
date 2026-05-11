# GraphQL API · GraphiQL Reference

Paste-ready reference for every Query and Mutation exposed by the merk
GraphQL API. Each operation block can be copied straight into GraphiQL
(or any GraphQL client); the matching variables JSON sits next to it.

---

## Endpoints

| Use                | URL                                                       |
| ------------------ | --------------------------------------------------------- |
| GraphiQL (browser) | `GET  /api/graphql` — interactive playground              |
| GraphQL HTTP       | `POST /api/graphql` — JSON `{ query, variables, operationName? }` |
| OpenAPI / REST     | `GET  /docs` — Scalar UI for the REST surface             |

**Subscriptions:** the Subscription root is live — `JobEvents` and
`DraftEvents` stream pipeline updates over WebSocket
(`/api/graphql/ws`). See §10 below.

---

## Authentication

Authenticated operations take a `Bearer` JWT in the request header:

```
Authorization: Bearer <token>
```

Get a token from `loginUser` or `registerUser`. In GraphiQL, paste it
into the **Request Headers** tab at the bottom of the page:

```json
{ "Authorization": "Bearer eyJhbGciOi..." }
```

Each operation below is tagged **🔒 auth** when a valid JWT is required.
Unauthenticated callers get a generic `Unauthorized` error.

---

## Schema entry points

```graphql
type Query {
  # User · profile · stats
  me                : MeResponseGql!         # 🔒
  myStats           : UserStatsGql!          # 🔒
  myReadingSessions : [ReadingSessionGql!]!  # 🔒
  myFollowing       : [AuthorGql!]!          # 🔒
  my2faStatus       : TwoFactorStatusGql!    # 🔒

  # Books · authors · taxonomy
  books             : [BookGql!]!
  book              : BookGql
  authors           : [BookAuthorGql!]!
  author            : BookAuthorGql
  categories        : [CategoryGql!]!
  booksByAuthor     : [BookGql!]!
  booksByCategory   : [BookGql!]!
  booksByTag        : [BookGql!]!

  # Chapters
  chapters          : [ChapterListItemGql!]!
  chapter           : ChapterGql

  # Reviews · highlights · comments
  bookReviews       : [BookReviewGql!]!
  chapterReviews    : [ChapterReviewGql!]!
  chapterHighlights : [HighlightGql!]!
  myHighlights      : [HighlightGql!]!       # 🔒
  chapterComments   : [CommentGql!]!
  highlightComments : [CommentGql!]!
  commentReplies    : [CommentGql!]!

  # Translations
  wordTranslations  : [WordTranslationGql!]!

  # Collections · bookmarks · goal · continue rail
  myCollections     : [CollectionGql!]!      # 🔒
  collection        : CollectionGql
  collectionBooks   : [CollectionBookGql!]!
  myBookmarks       : [BookmarkGql!]!        # 🔒
  myContinue        : [ContinueItemGql!]!    # 🔒
  myReadingGoal     : ReadingGoalGql         # 🔒

  # Admin (editor or admin role required) — F1-back
  ingestionJobs     : [IngestionJobGql!]!    # 🛡
  ingestionJob      : IngestionJobGql        # 🛡
  jobSteps          : [PipelineStepGql!]!    # 🛡
  jobLog            : [JobLogEntryGql!]!     # 🛡
  chapterDrafts     : [ChapterDraftGql!]!    # 🛡
  chapterDraft      : ChapterDraftGql        # 🛡
  aiModels          : [AiModelGql!]!         # 🛡
  adminUsage        : UsageOverviewGql!      # 🛡
  coverVariants     : [CoverVariantGql!]!    # 🛡
  publishChecks     : [PublishCheckGql!]!    # 🛡
  adminBooks        : [AdminBookGql!]!       # 🛡
  matchAuthors      : [AuthorMatchGql!]!     # 🛡
}

type Mutation {
  registerUser            : AuthPayload!
  loginUser               : AuthPayload!     # may return requires2fa = true
  login2faComplete        : AuthPayload!     # second leg of 2FA login
  logoutUser              : Boolean!         # 🔒
  forgotPassword          : Boolean!
  resetPasswordWithToken  : Boolean!
  updateProfile           : ProfileResponseGql! # 🔒
  changePassword          : Boolean!         # 🔒
  deleteMe                : Boolean!         # 🔒
  recordReadingSession    : ReadingSessionGql!  # 🔒
  setup2fa                : Setup2faPayload! # 🔒
  verify2fa               : Boolean!         # 🔒
  disable2fa              : Boolean!         # 🔒

  createBook              : BookGql!         # 🔒
  updateBook              : BookGql          # 🔒
  createAuthor            : BookAuthorGql!   # 🔒
  followAuthor            : Boolean!         # 🔒
  unfollowAuthor          : Boolean!         # 🔒

  createChapter           : ChapterGql!      # 🔒
  updateChapter           : ChapterGql       # 🔒

  createBookReview        : BookReviewGql!   # 🔒
  updateBookReview        : BookReviewGql    # 🔒
  deleteBookReview        : Boolean!         # 🔒
  voteBookReview          : Boolean!         # 🔒
  createChapterReview     : ChapterReviewGql!# 🔒
  voteChapterReview       : Boolean!         # 🔒
  flagReview              : Boolean!         # 🔒

  createHighlight         : HighlightGql!    # 🔒
  updateHighlight         : HighlightGql     # 🔒
  deleteHighlight         : Boolean!         # 🔒

  createComment           : CommentGql!      # 🔒
  updateComment           : CommentGql       # 🔒
  deleteComment           : Boolean!         # 🔒
  voteComment             : Boolean!         # 🔒

  submitTranslation       : WordTranslationGql! # 🔒
  voteTranslation         : Boolean!         # 🔒

  createCollection        : CollectionGql!   # 🔒
  updateCollection        : CollectionGql    # 🔒
  deleteCollection        : Boolean!         # 🔒
  addBookToCollection     : CollectionBookGql! # 🔒
  removeBookFromCollection: Boolean!         # 🔒
  upsertBookmark          : BookmarkGql!     # 🔒
  removeBookmark          : Boolean!         # 🔒
  upsertReadingGoal       : ReadingGoalGql!  # 🔒

  # Admin (editor or admin role required) — F1-back
  createIngestionJob       : IngestionJobGql! # 🛡
  startIngestionJob        : Boolean!         # 🛡  (kicks off pipeline worker)
  pauseIngestionJob        : Boolean!         # 🛡
  resumeIngestionJob       : Boolean!         # 🛡
  cancelIngestionJob       : Boolean!         # 🛡
  updateIngestionJobConfig : IngestionJobGql  # 🛡
  updateChapterDraft       : ChapterDraftGql  # 🛡
  approveChapterDraft      : Boolean!         # 🛡
  flagChapterDraft         : Boolean!         # 🛡
  rejectChapterDraft       : Boolean!         # 🛡
  reOcrChapterPages        : Boolean!         # 🛡
  generateCoverVariants    : [CoverVariantGql!]! # 🛡
  selectCoverVariant       : Boolean!         # 🛡
  publishIngestionJob      : PublishedBookGql! # 🛡
  updateAdminBook          : Boolean!         # 🛡
  unpublishBook            : Boolean!         # 🛡
}

type Subscription {
  jobEvents                : JobEventGql!     # 🛡  (WebSocket)
  draftEvents              : DraftEventGql!   # 🛡  (WebSocket)
}
```

Field names are `camelCase` in GraphQL (the Rust resolvers are
`snake_case`; async-graphql converts at schema-build time).
**🛡 markers** indicate admin/editor role required; resolved through
the migration-0002 RBAC graph.

---

## 1 · Auth & Account

### Register a new account

```graphql
mutation Register($username: String!, $email: String!, $password: String!) {
  registerUser(username: $username, email: $email, password: $password) {
    token
    user { id username email isActive isVerified planTier }
  }
}
```
```json
{ "username": "reader", "email": "user@example.com", "password": "OldPass1234!" }
```

### Log in

The login flow has two shapes. When the account has 2FA disabled,
`token` is the real access JWT. When 2FA is enabled, `token` is empty,
`requiresTwoFa` is `true`, and the client must follow up with
`login2faComplete` using the returned `challenge`.

```graphql
mutation Login($email: String!, $password: String!) {
  loginUser(email: $email, password: $password) {
    token
    requiresTwoFa
    challenge
    user { id username email isActive isVerified planTier }
  }
}
```
```json
{ "email": "user@example.com", "password": "OldPass1234!" }
```

### Complete 2FA login

```graphql
mutation Login2faComplete($challenge: String!, $code: String!) {
  login2faComplete(challenge: $challenge, code: $code) {
    token
    user { id username email planTier }
  }
}
```
```json
{ "challenge": "eyJhbGciOi...", "code": "123456" }
```

`code` is either a 6-digit TOTP from the authenticator app or one of the
recovery codes (`XXXXX-XXXXX`). Recovery codes are single-use.

### Log out 🔒

```graphql
mutation Logout {
  logoutUser
}
```

### Request a password-reset email

```graphql
mutation ForgotPassword($email: String!) {
  forgotPassword(email: $email)
}
```
```json
{ "email": "user@example.com" }
```

### Reset password with token

```graphql
mutation ResetPassword($token: String!, $newPassword: String!) {
  resetPasswordWithToken(token: $token, newPassword: $newPassword)
}
```
```json
{ "token": "a3f9...", "newPassword": "NewPass5678!" }
```

### Change current password 🔒

```graphql
mutation ChangePassword($oldPassword: String!, $newPassword: String!) {
  changePassword(oldPassword: $oldPassword, newPassword: $newPassword)
}
```

### Deactivate own account 🔒

```graphql
mutation DeleteMe {
  deleteMe
}
```

---

## 2 · Me · profile · plan · stats · sessions · following

### Current user, profile, and plan 🔒

```graphql
query Me {
  me {
    id
    username
    email
    isActive
    isVerified
    profile {
      id
      userId
      firstName
      lastName
      displayName
      avatarUrl
      bio
      language
      country
      timezone
      phone
      website
    }
    plan {
      tier        # "free" | "reader" | "patron"
      name        # "Musanif Free" / "Musanif Reader" / "Musanif Patron"
      features    # [String!]!
    }
  }
}
```

### Update profile 🔒

```graphql
mutation UpdateProfile($input: UpdateProfileInput!) {
  updateProfile(input: $input) {
    id userId firstName lastName displayName bio language country timezone phone website avatarUrl
  }
}
```
```json
{
  "input": {
    "displayName": "Strider",
    "bio": "Ranger of the North.",
    "language": "en",
    "country": "ME"
  }
}
```

### Reading stats grid 🔒

```graphql
query MyStats {
  myStats {
    booksReading
    booksCompleted
    booksReadLater
    booksDropped
    highlightsCount
    reviewsCount
    readingSessionsCount
    hoursRead          # sum(reading_session.duration_mins) / 60
    dayStreak          # consecutive UTC days with at least one session
  }
}
```

### Recent reading sessions 🔒

```graphql
query MyReadingSessions($limit: Int, $offset: Int) {
  myReadingSessions(limit: $limit, offset: $offset) {
    id bookId chapterId startedAt endedAt durationMins pageStart pageEnd device
  }
}
```
```json
{ "limit": 20, "offset": 0 }
```

### Authors I follow 🔒

```graphql
query MyFollowing($limit: Int, $offset: Int) {
  myFollowing(limit: $limit, offset: $offset) {
    id name slug bio avatarUrl website
  }
}
```

### 2FA setup, verify, disable, status 🔒

Setup is a three-step dance:

1. `setup2fa` returns the secret + otpauth URL (render as QR) + a fresh
   set of plaintext recovery codes. **Show the recovery codes to the
   user once and store them client-side until step 2 succeeds** — the
   server does not persist them in plaintext.
2. User enters the first 6-digit code from their authenticator app and
   the client passes it back along with the same `recoveryCodes` it just
   received. The server hashes the codes and flips `totpEnabledAt`.
3. From the next login on, the account is 2FA-protected.

```graphql
mutation Setup2fa {
  setup2fa {
    secret          # base32 — only returned once; usually rendered as a QR
    otpauthUrl      # otpauth://totp/Musanif:user@example.com?secret=...&issuer=Musanif
    recoveryCodes   # ["A2K7Q-MPRX9", ...] — single-use
  }
}

mutation Verify2fa($code: String!, $recoveryCodes: [String!]!) {
  verify2fa(code: $code, recoveryCodes: $recoveryCodes)
}
```
```json
{
  "code": "123456",
  "recoveryCodes": ["A2K7Q-MPRX9", "B8T3F-LHX24", "..."]
}
```

```graphql
mutation Disable2fa($code: String!) {
  disable2fa(code: $code)   # accepts a TOTP code or any unused recovery code
}

query My2faStatus {
  my2faStatus {
    enabled
    lastUsedAt
  }
}
```

### Record a reading session (heartbeat) 🔒

The reader posts a session on unmount or every ~60s. Side-effect: the
matching bookmark's `lastReadAt` and `lastChapter` are refreshed so the
Continue rail stays current.

```graphql
mutation RecordReadingSession($input: RecordReadingSessionInput!) {
  recordReadingSession(input: $input) {
    id bookId chapterId startedAt endedAt durationMins pageStart pageEnd device
  }
}
```
```json
{
  "input": {
    "bookSlug": "diwan-e-ghalib",
    "chapterSlug": "ghazal-no-1",
    "startedAt": "2026-05-06T18:30:00Z",
    "endedAt":   "2026-05-06T18:55:00Z",
    "durationMins": 25,
    "pageStart": 12,
    "pageEnd": 18,
    "device": "web"
  }
}
```

---

## 3 · Books · authors · taxonomy

### Search / list books

```graphql
query Books($filters: BookFiltersInput, $limit: Int, $offset: Int) {
  books(filters: $filters, limit: $limit, offset: $offset) {
    id title slug summary coverUrl language avgRating reviewCount chapterCount isPublished
  }
}
```
```json
{ "filters": { "q": "Ghalib", "lang": "ur" }, "limit": 20, "offset": 0 }
```

### Single book by slug

```graphql
query Book($slug: String!) {
  book(slug: $slug) {
    id title slug isbn summary description coverUrl pageCount language
    avgRating reviewCount chapterCount isPublished
  }
}
```

### Author search

```graphql
query Authors($q: String, $limit: Int, $offset: Int) {
  authors(q: $q, limit: $limit, offset: $offset) {
    id name slug bio avatarUrl website isFollowing
  }
}
```

### Author detail (with follow state) 🔒-aware

`isFollowing` resolves to `false` for unauthenticated requests; pass a
JWT to get the real flag.

```graphql
query Author($slug: String!) {
  author(slug: $slug) {
    id name slug bio avatarUrl website isFollowing
  }
}
```

### Categories

```graphql
query Categories {
  categories { id name slug description }
}
```

### Books by author / category / tag

```graphql
query BooksByAuthor($authorSlug: String!) {
  booksByAuthor(authorSlug: $authorSlug) {
    id title slug coverUrl avgRating chapterCount isPublished
  }
}

query BooksByCategory($slug: String!, $limit: Int, $offset: Int) {
  booksByCategory(slug: $slug, limit: $limit, offset: $offset) {
    id title slug coverUrl avgRating chapterCount isPublished
  }
}

query BooksByTag($slug: String!, $limit: Int, $offset: Int) {
  booksByTag(slug: $slug, limit: $limit, offset: $offset) {
    id title slug coverUrl avgRating chapterCount isPublished
  }
}
```

### Create a book / author 🔒

```graphql
mutation CreateBook($input: CreateBookInput!) {
  createBook(input: $input) {
    id title slug isbn summary language isPublished
  }
}
```
```json
{
  "input": {
    "title": "Diwan-e-Ghalib",
    "slug": "diwan-e-ghalib",
    "summary": "Selected ghazals.",
    "language": "ur"
  }
}
```

```graphql
mutation UpdateBook($slug: String!, $input: UpdateBookInput!) {
  updateBook(slug: $slug, input: $input) {
    id title summary description isPublished
  }
}

mutation CreateAuthor($input: CreateAuthorInput!) {
  createAuthor(input: $input) {
    id name slug bio
  }
}
```
```json
{ "input": { "name": "Mirza Ghalib", "slug": "mirza-ghalib", "bio": "Urdu poet, 1797–1869." } }
```

### Follow / unfollow an author 🔒

```graphql
mutation FollowAuthor($slug: String!) { followAuthor(slug: $slug) }
mutation UnfollowAuthor($slug: String!) { unfollowAuthor(slug: $slug) }
```

---

## 4 · Chapters

### List chapters in a book

```graphql
query Chapters($bookSlug: String!) {
  chapters(bookSlug: $bookSlug) {
    id number title slug summary readingTimeMins avgRating
  }
}
```

### Read a single chapter

```graphql
query Chapter($bookSlug: String!, $chapterSlug: String!) {
  chapter(bookSlug: $bookSlug, chapterSlug: $chapterSlug) {
    id bookId number title slug content contentFormat summary metaDescription
    wordCount readingTimeMins avgRating reviewCount isPublished
    prevChapter { number title slug }
    nextChapter { number title slug }
  }
}
```

### Create / update a chapter 🔒

```graphql
mutation CreateChapter($bookSlug: String!, $input: CreateChapterInput!) {
  createChapter(bookSlug: $bookSlug, input: $input) {
    id number title slug contentFormat
  }
}
```
```json
{
  "bookSlug": "diwan-e-ghalib",
  "input": {
    "number": 1,
    "slug": "ghazal-no-1",
    "title": "Ghazal No. 1",
    "content": "...",
    "contentFormat": "markdown",
    "summary": "Opening ghazal."
  }
}
```

```graphql
mutation UpdateChapter($bookSlug: String!, $chapterSlug: String!, $input: UpdateChapterInput!) {
  updateChapter(bookSlug: $bookSlug, chapterSlug: $chapterSlug, input: $input) {
    id title summary isPublished
  }
}
```

---

## 5 · Reviews

### Book reviews

```graphql
query BookReviews($bookSlug: String!, $limit: Int, $offset: Int, $spoilers: Boolean) {
  bookReviews(bookSlug: $bookSlug, limit: $limit, offset: $offset, spoilers: $spoilers) {
    id userId rating title body containsSpoiler readingStatus verifiedReader helpfulCount status
  }
}
```

### Chapter reviews

```graphql
query ChapterReviews($bookSlug: String!, $chapterSlug: String!, $limit: Int, $offset: Int) {
  chapterReviews(bookSlug: $bookSlug, chapterSlug: $chapterSlug, limit: $limit, offset: $offset) {
    id userId chapterId rating body containsSpoiler helpfulCount status
  }
}
```

### Create / update / delete / vote / flag 🔒

```graphql
mutation CreateBookReview($input: CreateBookReviewInput!) {
  createBookReview(input: $input) {
    id rating title body containsSpoiler readingStatus verifiedReader helpfulCount status
  }
}
```
```json
{
  "input": {
    "bookSlug": "diwan-e-ghalib",
    "rating": 5,
    "title": "A masterpiece",
    "body": "Layered, biting, eternal.",
    "containsSpoiler": false,
    "readingStatus": "completed"
  }
}
```

```graphql
mutation UpdateBookReview($reviewId: String!, $input: UpdateBookReviewInput!) {
  updateBookReview(reviewId: $reviewId, input: $input) {
    id rating title body
  }
}

mutation DeleteBookReview($reviewId: String!) { deleteBookReview(reviewId: $reviewId) }

mutation VoteBookReview($reviewId: String!, $value: Int!) {
  voteBookReview(reviewId: $reviewId, value: $value)
}

mutation CreateChapterReview($input: CreateChapterReviewInput!) {
  createChapterReview(input: $input) {
    id rating body helpfulCount status
  }
}

mutation VoteChapterReview($reviewId: String!, $value: Int!) {
  voteChapterReview(reviewId: $reviewId, value: $value)
}

mutation FlagReview($reviewId: String!, $reason: String!, $note: String) {
  flagReview(reviewId: $reviewId, reason: $reason, note: $note)
}
```

`value` is `1` (helpful) or `-1` (not helpful).

---

## 6 · Highlights

### Highlights in a chapter

```graphql
query ChapterHighlights($bookSlug: String!, $chapterSlug: String!, $public: Boolean) {
  chapterHighlights(bookSlug: $bookSlug, chapterSlug: $chapterSlug, public: $public) {
    id userId bookId chapterId offsetStart offsetEnd paragraph textSnapshot color note isPublic
  }
}
```

### My highlights, ordered 🔒

`order` is one of `created_desc` (default), `created_asc`,
`updated_desc`. Useful for the profile "Recent highlight" card and the
GraphiQL examples below.

```graphql
query MyHighlights($order: String, $limit: Int, $offset: Int) {
  myHighlights(order: $order, limit: $limit, offset: $offset) {
    id bookId chapterId offsetStart offsetEnd textSnapshot color note isPublic
  }
}
```
```json
{ "order": "created_desc", "limit": 1 }
```

### Create / update / delete 🔒

```graphql
mutation CreateHighlight($input: CreateHighlightInput!) {
  createHighlight(input: $input) {
    id offsetStart offsetEnd textSnapshot color note isPublic
  }
}
```
```json
{
  "input": {
    "bookSlug": "diwan-e-ghalib",
    "chapterSlug": "ghazal-no-1",
    "offsetStart": 120,
    "offsetEnd": 180,
    "paragraph": 3,
    "textSnapshot": "Hazaaron khwahishen aisi…",
    "color": "yellow",
    "note": "Couplet I keep coming back to.",
    "isPublic": false
  }
}
```

```graphql
mutation UpdateHighlight($highlightId: String!, $input: UpdateHighlightInput!) {
  updateHighlight(highlightId: $highlightId, input: $input) {
    id color note isPublic
  }
}

mutation DeleteHighlight($highlightId: String!) {
  deleteHighlight(highlightId: $highlightId)
}
```

---

## 7 · Comments

### List comments

```graphql
query ChapterComments($bookSlug: String!, $chapterSlug: String!, $limit: Int, $offset: Int) {
  chapterComments(bookSlug: $bookSlug, chapterSlug: $chapterSlug, limit: $limit, offset: $offset) {
    id userId chapterId highlightId parentId body isSpoiler isDeleted offsetStart offsetEnd textSnapshot
  }
}

query HighlightComments($highlightId: String!) {
  highlightComments(highlightId: $highlightId) {
    id userId body isSpoiler isDeleted parentId
  }
}

query CommentReplies($parentId: String!) {
  commentReplies(parentId: $parentId) {
    id userId body isSpoiler isDeleted
  }
}
```

### Write / edit / delete / vote 🔒

```graphql
mutation CreateComment($input: CreateCommentInput!) {
  createComment(input: $input) {
    id body isSpoiler parentId highlightId
  }
}
```
```json
{
  "input": {
    "bookSlug": "diwan-e-ghalib",
    "chapterSlug": "ghazal-no-1",
    "body": "The radif here is doing real work.",
    "isSpoiler": false
  }
}
```

```graphql
mutation UpdateComment($commentId: String!, $body: String!) {
  updateComment(commentId: $commentId, body: $body) {
    id body
  }
}

mutation DeleteComment($commentId: String!) {
  deleteComment(commentId: $commentId)
}

mutation VoteComment($commentId: String!, $value: Int!) {
  voteComment(commentId: $commentId, value: $value)
}
```

---

## 8 · Translations

### Look up word translations

Priority order: chapter scope → book scope → global. Pass `bookSlug` /
`chapterSlug` to scope the lookup.

```graphql
query WordTranslations($word: String!, $targetLang: String!, $bookSlug: String, $chapterSlug: String) {
  wordTranslations(word: $word, targetLang: $targetLang, bookSlug: $bookSlug, chapterSlug: $chapterSlug) {
    id word translation sourceLang targetLang submittedBy scope
    bookId chapterId contextNote upvotes downvotes score
  }
}
```
```json
{ "word": "ishq", "targetLang": "en", "bookSlug": "diwan-e-ghalib", "chapterSlug": "ghazal-no-1" }
```

### Submit / vote 🔒

```graphql
mutation SubmitTranslation($input: CreateTranslationInput!) {
  submitTranslation(input: $input) {
    id word translation sourceLang targetLang scope upvotes downvotes
  }
}
```
```json
{
  "input": {
    "word": "ishq",
    "translation": "love (consuming)",
    "sourceLang": "ur",
    "targetLang": "en",
    "scope": "chapter",
    "bookSlug": "diwan-e-ghalib",
    "chapterSlug": "ghazal-no-1",
    "contextNote": "Distinguished from mohabbat in classical usage."
  }
}
```

```graphql
mutation VoteTranslation($translationId: String!, $value: Int!) {
  voteTranslation(translationId: $translationId, value: $value)
}
```

---

## 9 · Collections · bookmarks · goal · Continue rail

### My collections 🔒

```graphql
query MyCollections($limit: Int, $offset: Int) {
  myCollections(limit: $limit, offset: $offset) {
    id userId name description coverUrl isPublic
  }
}

query Collection($id: String!) {
  collection(id: $id) {
    id userId name description coverUrl isPublic
  }
}

query CollectionBooks($collectionId: String!, $limit: Int, $offset: Int) {
  collectionBooks(collectionId: $collectionId, limit: $limit, offset: $offset) {
    bookId position note
  }
}
```

### Manage collections 🔒

```graphql
mutation CreateCollection($input: CreateCollectionInput!) {
  createCollection(input: $input) {
    id name description isPublic
  }
}
```
```json
{ "input": { "name": "Re-read in 2026", "description": "Books worth a second pass.", "isPublic": false } }
```

```graphql
mutation UpdateCollection($id: String!, $input: UpdateCollectionInput!) {
  updateCollection(id: $id, input: $input) {
    id name description isPublic
  }
}

mutation DeleteCollection($id: String!) {
  deleteCollection(id: $id)
}

mutation AddBookToCollection($collectionId: String!, $input: AddBookInput!) {
  addBookToCollection(collectionId: $collectionId, input: $input) {
    bookId position note
  }
}

mutation RemoveBookFromCollection($collectionId: String!, $bookSlug: String!) {
  removeBookFromCollection(collectionId: $collectionId, bookSlug: $bookSlug)
}
```
```json
{ "collectionId": "abc123", "input": { "bookSlug": "diwan-e-ghalib", "position": 1, "note": "Start here." } }
```

### My shelf (bookmarks) 🔒

`status` filters to `reading | readlater | completed | dropped`. `order`
is one of `updated_at_desc` (default), `completed_at_desc`,
`last_read_at_desc`, `created_asc`, `created_desc` — useful for the
profile "Recently finished" rail.

```graphql
query MyBookmarks($status: String, $order: String, $limit: Int, $offset: Int) {
  myBookmarks(status: $status, order: $order, limit: $limit, offset: $offset) {
    id bookId status progress notes
    lastChapterId lastOffset lastReadAt progressPct
  }
}
```
```json
{ "status": "completed", "order": "completed_at_desc", "limit": 10 }
```

### Continue Reading rail 🔒

Composes book + last_chapter + bookmark progress and computes
`timeLeftMins` = `chapter.readingTimeMins * (1 - progressPct/100)`.

```graphql
query MyContinue($limit: Int) {
  myContinue(limit: $limit) {
    bookId bookSlug bookTitle coverUrl
    lastChapterId lastChapterSlug lastChapterTitle lastChapterNumber
    progressPct timeLeftMins lastReadAt
  }
}
```
```json
{ "limit": 3 }
```

### Reading goal 🔒

`paceHint` is server-rendered (locale-neutral for now) and derived from
`target`, `completed`, day-of-year. `onTrack` mirrors the chip color the
UI shows.

```graphql
query MyReadingGoal($year: Int) {
  myReadingGoal(year: $year) {
    id year target completed progressPct onTrack paceHint
  }
}

mutation UpsertReadingGoal($year: Int!, $target: Int!) {
  upsertReadingGoal(year: $year, target: $target) {
    id year target completed progressPct onTrack paceHint
  }
}
```
```json
{ "year": 2026, "target": 24 }
```

### Bookmark a book / remove 🔒

```graphql
mutation UpsertBookmark($bookSlug: String!, $input: UpsertBookmarkInput!) {
  upsertBookmark(bookSlug: $bookSlug, input: $input) {
    id bookId status progress notes lastReadAt progressPct
  }
}

mutation RemoveBookmark($bookSlug: String!) {
  removeBookmark(bookSlug: $bookSlug)
}
```
```json
{
  "bookSlug": "diwan-e-ghalib",
  "input": { "status": "reading", "progress": 24, "notes": "Picking up after the qasidas." }
}
```

---

## Common variables block

GraphiQL keeps headers and variables in separate panels at the bottom
of the screen. A typical authenticated session sets:

```jsonc
// Variables panel
{
  "limit": 20,
  "offset": 0
}

// Request Headers panel
{
  "Authorization": "Bearer eyJhbGciOi..."
}
```

---

## 10 · Admin pipeline (F1-back) 🛡

Every operation here requires the `editor` or `admin` role. Assign
roles via the RBAC graph: `RELATE user:foo->assigned_role->role:editor`.

### Upload + create job

File upload itself is REST (multipart isn't a great fit for GraphQL):
- `POST /api/v1/admin/uploads` — body is the raw file, headers carry
  `X-Filename` and `Content-Type`. Returns the registered
  `uploaded_asset` with `id`.
- *(GCS only, future)* `POST /api/v1/admin/uploads/sign` →
  `{ upload_url, bucket, object, headers }`; client PUTs directly,
  then `POST /api/v1/admin/uploads/{id}/finalize` registers the asset.

Then in GraphQL:

```graphql
mutation CreateIngestionJob($input: CreateIngestionJobInput!) {
  createIngestionJob(input: $input) {
    id assetId stage status aiProvider aiModel
  }
}
```
```json
{
  "input": {
    "assetId": "uploaded_asset:abc123",
    "hintTitle": "Diwan-e-Ghalib",
    "hintAuthor": "Mirza Ghalib"
  }
}
```

### Drive the pipeline

```graphql
mutation StartIngestionJob($id: String!) { startIngestionJob(id: $id) }
mutation PauseIngestionJob($id: String!) { pauseIngestionJob(id: $id) }
mutation ResumeIngestionJob($id: String!) { resumeIngestionJob(id: $id) }
mutation CancelIngestionJob($id: String!) { cancelIngestionJob(id: $id) }

mutation UpdateIngestionJobConfig($id: String!, $input: UpdateJobConfigInput!) {
  updateIngestionJobConfig(id: $id, input: $input) {
    id aiProvider aiModel
  }
}
```
```json
{ "id": "ingestion_job:xyz", "input": { "aiProvider": "claude", "aiModel": "claude-sonnet-4-6" } }
```

### Inspect job state

```graphql
query IngestionJobs($status: String, $stage: Int, $limit: Int) {
  ingestionJobs(status: $status, stage: $stage, limit: $limit) {
    id stage status hintTitle hintAuthor aiProvider aiModel tokensUsed estCostUsd
  }
}

query IngestionJob($id: String!) {
  ingestionJob(id: $id) {
    id assetId bookId hintTitle hintAuthor pages stage status
    aiProvider aiModel overallConfidence chaptersTotal chaptersFlagged
    tokensUsed estCostUsd coverColor coverGlyph
  }
}

query JobSteps($job: String!) {
  jobSteps(job: $job) { id n label status detail startedAt finishedAt }
}

query JobLog($job: String!, $since: String, $limit: Int) {
  jobLog(job: $job, since: $since, limit: $limit) { id t kind message }
}
```

### Chapter drafts (Review + Edit)

```graphql
query ChapterDrafts($job: String!) {
  chapterDrafts(job: $job) {
    id n titleUr titleEn pageRange status confidence flagReason
    aiSummary themes entities
  }
}

query ChapterDraft($id: String!) {
  chapterDraft(id: $id) {
    id n titleUr titleEn pageRange aiContent humanContent aiContentFormat
    aiSummary themes entities confidence status flagReason
    approvedById approvedAt pagesReOcrCount
  }
}

mutation UpdateChapterDraft($id: String!, $input: UpdateChapterDraftInput!) {
  updateChapterDraft(id: $id, input: $input) {
    id titleUr titleEn pageRange humanContent
  }
}

mutation ApproveChapterDraft($id: String!) { approveChapterDraft(id: $id) }
mutation FlagChapterDraft($id: String!, $reason: String!) { flagChapterDraft(id: $id, reason: $reason) }
mutation RejectChapterDraft($id: String!) { rejectChapterDraft(id: $id) }
mutation ReOcrChapterPages($id: String!, $pages: String!) { reOcrChapterPages(id: $id, pages: $pages) }
```

### AI providers + cost

```graphql
query AiModels {
  aiModels {
    id provider name label note inputCostPerMillion outputCostPerMillion isActive
  }
}

query AdminUsage($period: String) {
  adminUsage(period: $period) {
    period tokensUsed estCostUsd monthlyBudgetUsd budgetUsedPct
    byModel { modelId modelLabel tokensUsed costUsd }
  }
}
```
```json
{ "period": "month" }
```

### Cover variants

```graphql
mutation GenerateCoverVariants($job: String!, $prompt: String) {
  generateCoverVariants(job: $job, prompt: $prompt) {
    id bucket object isSelected
  }
}

query CoverVariants($job: String!) {
  coverVariants(job: $job) { id bucket object modelId prompt isSelected }
}

mutation SelectCoverVariant($job: String!, $variant: String!) {
  selectCoverVariant(job: $job, variant: $variant)
}
```

### Publish

```graphql
query PublishChecks($job: String!) {
  publishChecks(job: $job) { ok gate label detail }
}

mutation PublishIngestionJob($job: String!, $input: PublishInput!) {
  publishIngestionJob(job: $job, input: $input) {
    id title slug coverUrl chapterCount
  }
}
```
```json
{
  "job": "ingestion_job:xyz",
  "input": { "visibility": "public", "scheduleAt": null }
}
```

### Admin library

```graphql
query AdminBooks($visibility: String, $isPublished: Boolean, $limit: Int) {
  adminBooks(visibility: $visibility, isPublished: $isPublished, limit: $limit) {
    id title slug visibility isPublished chapterCount avgRating
  }
}

mutation UpdateAdminBook($slug: String!, $visibility: String!) {
  updateAdminBook(slug: $slug, visibility: $visibility)
}

mutation UnpublishBook($slug: String!) { unpublishBook(slug: $slug) }
```

### Author resolution

```graphql
query MatchAuthors($name: String!) {
  matchAuthors(name: $name) { id name slug bio confidence }
}
```

### PDF page preview (REST)

The Review pane renders the original page next to the AI extraction:

```
GET /api/v1/admin/ingestion-jobs/{job_id}/pages/{n}
Authorization: Bearer <token>
```

Returns binary `image/webp` (or `image/png`); the response is cacheable
for 5 minutes.

---

## 11 · Subscriptions (F3-back) 🛡

Live transport: WebSocket at `/api/graphql/ws` using the
`graphql-transport-ws` sub-protocol. Auth token rides in the
`connection_init` payload; the client should send:

```json
{ "type": "connection_init", "payload": { "Authorization": "Bearer eyJ..." } }
```

### Job events

Streams `step_update`, `log_entry`, `chapter_draft_added`, and
`pipeline_completed` for one job. Each event sets `kind` to identify
its variant; the rest of the fields are populated only for that variant.

```graphql
subscription JobEvents($job: String!) {
  jobEvents(job: $job) {
    kind
    jobId
    # step_update
    n label status detail startedAt finishedAt
    # log_entry
    t logKind message
    # chapter_draft_added
    draftId
  }
}
```
```json
{ "job": "ingestion_job:xyz" }
```

### Draft events

```graphql
subscription DraftEvents($job: String!) {
  draftEvents(job: $job) {
    kind        # "draft_updated" | "draft_approved" | "draft_flagged"
    jobId draftId
    byUser      # populated for draft_approved
    reason      # populated for draft_flagged
  }
}
```

**Polling fallback.** Clients without WebSocket support poll
`jobSteps` / `jobLog` / `chapterDrafts` every 2 seconds. Same data,
just chattier.

---

## Cross-references

- `docs/schema/05_graphql.md` — full type/input/enum definitions
- `docs/schema/06_http_examples.md` — REST surface examples
- `docs/api-and-db-plan.md` §1 — transport policy (REST is frozen for
  cacheable GETs / file upload / binary; GraphQL gets new work)
- `docs/api-and-db-plan.md` §3 — admin pipeline design
- `docs/api-and-db-plan.md` §5 — subscriptions design
