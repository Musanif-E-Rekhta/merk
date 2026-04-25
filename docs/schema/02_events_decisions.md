# SurrealDB Events & Key Design Decisions

## SurrealDB Event Summary

| Event | Trigger | Effect |
|---|---|---|
| `book_review_stats` | `book_review` CREATE / UPDATE / DELETE | Recalculates `book.avg_rating`, `book.review_count` |
| `chapter_review_stats` | `chapter_review` CREATE / UPDATE / DELETE | Recalculates `chapter.avg_rating`, `chapter.review_count` |
| `book_review_helpful_count` | `book_review_vote` CREATE / DELETE | Syncs `book_review.helpful_count` |
| `translation_vote_counts` | `translation_vote` CREATE / DELETE | Syncs `word_translation.upvotes`, `.downvotes` |

---

## Key Design Decisions

| Decision | Reason |
|---|---|
| `wrote`, `bookmark`, `follows` as RELATION tables | Enables SurrealDB graph traversal: `SELECT ->wrote->book FROM author:x` |
| Tags/categories embedded as arrays on `book` | Fast reads for display; graph traversal only needed when filtering |
| `chapter.slug` unique per book, not globally | `/books/a/chapters/prologue` and `/books/b/chapters/prologue` are both valid |
| `comment.deleted_at` soft delete | Deleting a parent would orphan the reply thread |
| `review_flag.review` typed as `record` (no table constraint) | Allows flagging both `book_review` and `chapter_review` without two flag tables |
| `avg_rating` / `review_count` denormalized on `book` and `chapter` | Maintained by SurrealDB events; avoids aggregation query on every page load |
| `text_snapshot` on highlight and comment | Character offsets break on content edits; snapshot preserves what the user selected |
| `verified_reader` written at review creation time | Avoids a bookmark join on every review read |
| `rating_dist` object on book | Enables histogram render without GROUP BY per request |
