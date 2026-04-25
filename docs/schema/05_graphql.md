# GraphQL Schema

Implemented with `async-graphql` (Rust). All list fields use `limit`/`offset` pagination. Pass `Authorization: Bearer <token>` for auth-required operations.

GraphQL endpoint: `POST /api/graphql` — GraphiQL UI at `GET /api/graphql`.

```graphql
# ── Core types ────────────────────────────────────────────────────────────────
type User {
  id:         ID!
  username:   String!
  email:      String!
  isActive:   Boolean!
  isVerified: Boolean!
}

type Profile {
  id:          ID!
  userId:      String!
  firstName:   String
  lastName:    String
  displayName: String
  avatarUrl:   String
  bio:         String
  language:    String!
  country:     String!
  timezone:    String
  phone:       String
  website:     String
}

type Me {
  id:         ID!
  username:   String!
  email:      String!
  isActive:   Boolean!
  isVerified: Boolean!
  profile:    Profile
}

type UserStats {
  booksReading:          Int!
  booksCompleted:        Int!
  booksReadLater:        Int!
  booksDropped:          Int!
  highlightsCount:       Int!
  reviewsCount:          Int!
  readingSessionsCount:  Int!
}

type ReadingSession {
  id:           ID!
  bookId:       String!
  chapterId:    String
  startedAt:    String
  endedAt:      String
  durationMins: Int
  pageStart:    Int!
  pageEnd:      Int
  device:       String
}

type AuthPayload {
  token: String!
  user:  User!
}

type Author {
  id:        ID!
  name:      String!
  slug:      String!
  bio:       String
  avatarUrl: String
  website:   String
}

type Category {
  id:          ID!
  name:        String!
  slug:        String!
  description: String
}

type Tag {
  id:   ID!
  name: String!
  slug: String!
}

type Book {
  id:           ID!
  title:        String!
  slug:         String!
  isbn:         String
  summary:      String
  description:  String
  coverUrl:     String
  pageCount:    Int
  language:     String!
  avgRating:    Float
  reviewCount:  Int!
  chapterCount: Int!
  isPublished:  Boolean!
}

type ChapterNav {
  number: Int!
  title:  String
  slug:   String!
}

type Chapter {
  id:              ID!
  bookId:          String!
  number:          Int!
  title:           String
  slug:            String!
  content:         String!
  contentFormat:   String!
  summary:         String
  metaDescription: String
  wordCount:       Int
  readingTimeMins: Int
  avgRating:       Float
  reviewCount:     Int!
  isPublished:     Boolean!
  prevChapter:     ChapterNav
  nextChapter:     ChapterNav
}

type BookReview {
  id:              ID!
  userId:          String!
  bookId:          String!
  rating:          Int!
  title:           String
  body:            String
  containsSpoiler: Boolean!
  readingStatus:   String!
  verifiedReader:  Boolean!
  helpfulCount:    Int!
  status:          String!
}

type ChapterReview {
  id:              ID!
  userId:          String!
  chapterId:       String!
  rating:          Int!
  body:            String
  containsSpoiler: Boolean!
  helpfulCount:    Int!
  status:          String!
}

type Highlight {
  id:           ID!
  userId:       String!
  chapterId:    String!
  bookId:       String!
  offsetStart:  Int!
  offsetEnd:    Int!
  paragraph:    Int!
  textSnapshot: String!
  color:        String!
  note:         String
  isPublic:     Boolean!
}

type Comment {
  id:           ID!
  userId:       String!
  chapterId:    String!
  highlightId:  String
  parentId:     String
  body:         String!
  isSpoiler:    Boolean!
  isDeleted:    Boolean!
  offsetStart:  Int
  offsetEnd:    Int
  textSnapshot: String
}

type WordTranslation {
  id:          ID!
  word:        String!
  translation: String!
  sourceLang:  String!
  targetLang:  String!
  submittedBy: String!
  scope:       String!      # "global" | "book" | "chapter"
  bookId:      String
  chapterId:   String
  contextNote: String
  upvotes:     Int!
  downvotes:   Int!
  score:       Int!
}

type Collection {
  id:          ID!
  userId:      String!
  name:        String!
  description: String
  coverUrl:    String
  isPublic:    Boolean!
}

type CollectionBook {
  bookId:   String!
  position: Int
  note:     String
}

type Bookmark {
  id:       ID!
  bookId:   String!
  status:   String!         # "reading" | "readlater" | "completed" | "dropped"
  progress: Int
  notes:    String
}

type ReadingGoal {
  id:          ID!
  year:        Int!
  target:      Int!
  completed:   Int!
  progressPct: Float!
}

# ── Queries ───────────────────────────────────────────────────────────────────
type Query {
  # Current user (auth)
  me: Me
  myStats: UserStats                                                             # auth
  myReadingSessions(limit: Int, offset: Int): [ReadingSession!]!                # auth
  myFollowing(limit: Int, offset: Int): [Author!]!                              # auth

  # Books & taxonomy
  books(q: String, lang: String, limit: Int, offset: Int): [Book!]!
  book(slug: String!): Book
  booksByAuthor(slug: String!, limit: Int, offset: Int): [Book!]!
  booksByCategory(slug: String!, limit: Int, offset: Int): [Book!]!
  booksByTag(slug: String!, limit: Int, offset: Int): [Book!]!
  authors(q: String, limit: Int, offset: Int): [Author!]!
  author(slug: String!): Author
  categories: [Category!]!

  # Chapters
  chapters(bookSlug: String!, limit: Int, offset: Int): [Chapter!]!   # table of contents
  chapter(bookSlug: String!, chapterSlug: String!): Chapter

  # Reviews
  bookReviews(bookSlug: String!, limit: Int, offset: Int): [BookReview!]!
  chapterReviews(bookSlug: String!, chapterSlug: String!, limit: Int, offset: Int): [ChapterReview!]!

  # Highlights
  chapterHighlights(bookSlug: String!, chapterSlug: String!, publicOnly: Boolean, limit: Int, offset: Int): [Highlight!]!
  myHighlights(limit: Int, offset: Int): [Highlight!]!                  # auth

  # Comments
  chapterComments(bookSlug: String!, chapterSlug: String!, limit: Int, offset: Int): [Comment!]!
  highlightComments(highlightId: ID!, limit: Int, offset: Int): [Comment!]!
  commentReplies(parentId: ID!): [Comment!]!

  # Translations (priority: chapter → book → global)
  wordTranslations(word: String!, targetLang: String!, bookSlug: String, chapterSlug: String): [WordTranslation!]!

  # Collections & shelf (auth)
  myCollections(limit: Int, offset: Int): [Collection!]!
  collection(id: ID!): Collection
  collectionBooks(collectionId: ID!, limit: Int, offset: Int): [CollectionBook!]!
  myBookmarks(status: String, limit: Int, offset: Int): [Bookmark!]!
  myReadingGoal(year: Int): ReadingGoal
}

# ── Mutations ─────────────────────────────────────────────────────────────────
type Mutation {
  # Auth
  registerUser(username: String!, email: String!, password: String!): AuthPayload!
  loginUser(email: String!, password: String!): AuthPayload!
  logoutUser: Boolean!                                                          # auth
  forgotPassword(email: String!): Boolean!
  resetPasswordWithToken(token: String!, newPassword: String!): Boolean!

  # Profile management (auth)
  updateProfile(input: UpdateProfileInput!): Profile!
  changePassword(oldPassword: String!, newPassword: String!): Boolean!
  deleteMe: Boolean!

  # Books & authors (auth)
  createBook(input: CreateBookInput!): Book!
  updateBook(slug: String!, input: UpdateBookInput!): Book
  createAuthor(input: CreateAuthorInput!): Author!
  followAuthor(slug: String!): Boolean!
  unfollowAuthor(slug: String!): Boolean!

  # Chapters (auth)
  createChapter(input: CreateChapterInput!): Chapter!
  updateChapter(slug: String!, bookSlug: String!, input: UpdateChapterInput!): Chapter

  # Reviews (auth)
  createBookReview(input: CreateBookReviewInput!): BookReview!
  updateBookReview(id: ID!, input: UpdateBookReviewInput!): BookReview
  deleteBookReview(id: ID!): Boolean!
  voteBookReview(id: ID!, value: Int!): BookReview!                      # value: 1 or -1
  createChapterReview(input: CreateChapterReviewInput!): ChapterReview!
  voteChapterReview(id: ID!, value: Int!): ChapterReview!
  flagReview(reviewId: ID!, reason: String!, note: String): Boolean!

  # Highlights (auth)
  createHighlight(input: CreateHighlightInput!): Highlight!
  updateHighlight(id: ID!, input: UpdateHighlightInput!): Highlight
  deleteHighlight(highlightId: ID!): Boolean!

  # Comments (auth)
  createComment(input: CreateCommentInput!): Comment!
  updateComment(id: ID!, input: UpdateCommentInput!): Comment
  deleteComment(commentId: ID!): Boolean!
  voteComment(id: ID!, value: Int!): Comment!                            # value: 1 or -1

  # Translations (auth)
  submitTranslation(input: CreateTranslationInput!): WordTranslation!
  voteTranslation(id: ID!, value: Int!): WordTranslation!

  # Collections & shelf (auth)
  createCollection(input: CreateCollectionInput!): Collection!
  updateCollection(id: ID!, input: UpdateCollectionInput!): Collection
  deleteCollection(id: ID!): Boolean!
  addBookToCollection(collectionId: ID!, input: AddBookInput!): CollectionBook!
  removeBookFromCollection(collectionId: ID!, bookSlug: String!): Boolean!
  upsertBookmark(input: UpsertBookmarkInput!): Bookmark!
  removeBookmark(bookSlug: String!): Boolean!
  upsertReadingGoal(year: Int!, target: Int!): ReadingGoal!
}

# ── Input types ───────────────────────────────────────────────────────────────
input UpdateProfileInput {
  firstName:   String
  lastName:    String
  displayName: String
  avatarUrl:   String
  bio:         String
  language:    String
  country:     String
  timezone:    String
  phone:       String
  website:     String
}

input CreateBookInput {
  title:         String!
  slug:          String!
  isbn:          String
  summary:       String
  description:   String
  coverUrl:      String
  pageCount:     Int
  language:      String
  publisherSlug: String
}

input UpdateBookInput {
  title:       String
  summary:     String
  description: String
  coverUrl:    String
  pageCount:   Int
  isPublished: Boolean
}

input CreateAuthorInput {
  name:      String!
  slug:      String!
  bio:       String
  avatarUrl: String
  website:   String
}

input CreateChapterInput {
  bookSlug:        String!
  number:          Int!
  title:           String
  slug:            String!
  content:         String!
  contentFormat:   String
  summary:         String
  metaDescription: String
  wordCount:       Int
}

input UpdateChapterInput {
  title:           String
  content:         String
  summary:         String
  metaDescription: String
  wordCount:       Int
  isPublished:     Boolean
}

input CreateBookReviewInput {
  bookSlug:        String!
  rating:          Int!
  title:           String
  body:            String
  containsSpoiler: Boolean
  readingStatus:   String!
}

input UpdateBookReviewInput {
  rating:          Int
  title:           String
  body:            String
  containsSpoiler: Boolean
  readingStatus:   String
}

input CreateChapterReviewInput {
  bookSlug:        String!
  chapterSlug:     String!
  rating:          Int!
  body:            String
  containsSpoiler: Boolean
}

input CreateHighlightInput {
  bookSlug:     String!
  chapterSlug:  String!
  offsetStart:  Int!
  offsetEnd:    Int!
  paragraph:    Int!
  textSnapshot: String!
  color:        String
  note:         String
  isPublic:     Boolean
}

input UpdateHighlightInput {
  color:    String
  note:     String
  isPublic: Boolean
}

input CreateCommentInput {
  bookSlug:     String!
  chapterSlug:  String!
  highlightId:  ID
  parentId:     ID
  body:         String!
  isSpoiler:    Boolean
  offsetStart:  Int
  offsetEnd:    Int
  textSnapshot: String
}

input UpdateCommentInput {
  body: String!
}

input CreateTranslationInput {
  word:        String!
  translation: String!
  sourceLang:  String!
  targetLang:  String!
  scope:       String!
  bookSlug:    String
  chapterSlug: String
  contextNote: String
}

input CreateCollectionInput {
  name:        String!
  description: String
  isPublic:    Boolean
}

input UpdateCollectionInput {
  name:        String
  description: String
  isPublic:    Boolean
}

input AddBookInput {
  bookSlug: String!
  position: Int
  note:     String
}

input UpsertBookmarkInput {
  bookSlug:  String!
  status:    String!
  progress:  Int
  notes:     String
}
```
