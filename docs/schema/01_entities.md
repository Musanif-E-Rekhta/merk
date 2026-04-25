# Entity Overview & Table Definitions

## Entity Overview

| Group | Tables |
|---|---|
| **Identity** | `user`, `profile` *(existing)* |
| **Content** | `author`, `publisher`, `category`, `tag`, `book`, `chapter` |
| **Graph Relations** | `wrote`, `follows`, `bookmark`, `collection_book` |
| **Reading** | `reading_session`, `reading_goal`, `collection` |
| **Annotations** | `highlight`, `comment`, `comment_vote` |
| **Reviews** | `book_review`, `chapter_review`, `book_review_vote`, `chapter_review_vote`, `review_flag` |
| **Translations** | `word_translation`, `translation_vote` |

Total: **25 tables** across 7 groups.

---

## Table Definitions

### `author`
| Field | Type | Notes |
|---|---|---|
| `id` | record | SurrealDB auto |
| `name` | string | |
| `slug` | string | UNIQUE |
| `bio` | option\<string\> | |
| `avatar_url` | option\<string\> | |
| `website` | option\<string\> | |
| `born_at` | option\<datetime\> | |
| `created_at` | datetime | READONLY |
| `updated_at` | datetime | auto |

### `publisher`
| Field | Type | Notes |
|---|---|---|
| `id` | record | |
| `name` | string | UNIQUE |
| `website` | option\<string\> | |
| `country` | option\<string\> | |
| `created_at` | datetime | READONLY |

### `category`
| Field | Type | Notes |
|---|---|---|
| `id` | record | |
| `name` | string | |
| `slug` | string | UNIQUE |
| `description` | option\<string\> | |
| `parent` | option\<record\<category\>\> | null = top-level |
| `created_at` | datetime | READONLY |

### `tag`
| Field | Type | Notes |
|---|---|---|
| `id` | record | |
| `name` | string | |
| `slug` | string | UNIQUE |
| `created_at` | datetime | READONLY |

### `book`
| Field | Type | Notes |
|---|---|---|
| `id` | record | |
| `title` | string | |
| `slug` | string | UNIQUE |
| `isbn` | option\<string\> | UNIQUE |
| `summary` | option\<string\> | 2–3 sentences, reader-facing |
| `description` | option\<string\> | full synopsis |
| `cover_url` | option\<string\> | |
| `page_count` | option\<int\> | |
| `language` | string | default `"en"` |
| `published_at` | option\<datetime\> | |
| `publisher` | option\<record\<publisher\>\> | |
| `categories` | array\<record\<category\>\> | embedded, fast reads |
| `tags` | array\<record\<tag\>\> | embedded |
| `avg_rating` | option\<float\> | maintained by event |
| `review_count` | int | maintained by event |
| `chapter_count` | int | maintained by app |
| `rating_dist` | object | `{"1":0,"2":0,"3":0,"4":0,"5":0}` |
| `is_published` | bool | |
| `created_at` | datetime | READONLY |
| `updated_at` | datetime | auto |

**Indexes:** `slug` (UNIQUE), `isbn` (UNIQUE), fulltext on `title + summary`

### `chapter`
| Field | Type | Notes |
|---|---|---|
| `id` | record | |
| `book` | record\<book\> | |
| `number` | int | order within book |
| `title` | option\<string\> | |
| `slug` | string | unique per book |
| `content` | string | full text |
| `content_format` | string | `markdown \| html \| plaintext` |
| `summary` | option\<string\> | reader-facing blurb |
| `meta_description` | option\<string\> | ≤160 chars, SEO |
| `word_count` | option\<int\> | |
| `reading_time_mins` | option\<int\> | ceil(word\_count / 238) |
| `avg_rating` | option\<float\> | maintained by event |
| `review_count` | int | maintained by event |
| `is_published` | bool | |
| `published_at` | option\<datetime\> | |
| `created_at` | datetime | READONLY |
| `updated_at` | datetime | auto — drives ETag |

**Indexes:** `(book, number)` UNIQUE, `(book, slug)` UNIQUE, fulltext on `title + content + summary` with HIGHLIGHTS

### `wrote` *(graph relation: author → book)*
| Field | Type | Notes |
|---|---|---|
| `in` | record\<author\> | |
| `out` | record\<book\> | |
| `role` | string | `author \| co_author \| illustrator \| editor \| translator` |
| `created_at` | datetime | READONLY |

**Index:** `(in, out, role)` UNIQUE

### `follows` *(graph relation: user → author)*
| Field | Type | Notes |
|---|---|---|
| `in` | record\<user\> | |
| `out` | record\<author\> | |
| `created_at` | datetime | READONLY |

**Index:** `(in, out)` UNIQUE

### `bookmark` *(graph relation: user → book)*
| Field | Type | Notes |
|---|---|---|
| `in` | record\<user\> | |
| `out` | record\<book\> | |
| `status` | string | `reading \| readlater \| completed \| dropped` |
| `progress` | option\<int\> | current page |
| `notes` | option\<string\> | private note |
| `started_at` | option\<datetime\> | |
| `completed_at` | option\<datetime\> | |
| `created_at` | datetime | READONLY |
| `updated_at` | datetime | auto |

**Index:** `(in, out)` UNIQUE — one bookmark per user/book pair

### `reading_session`
| Field | Type | Notes |
|---|---|---|
| `id` | record | |
| `user` | record\<user\> | |
| `book` | record\<book\> | |
| `chapter` | option\<record\<chapter\>\> | |
| `started_at` | datetime | READONLY |
| `ended_at` | option\<datetime\> | |
| `duration_mins` | option\<int\> | |
| `page_start` | int | |
| `page_end` | option\<int\> | |
| `device` | option\<string\> | `web \| mobile \| ereader` |

### `reading_goal`
| Field | Type | Notes |
|---|---|---|
| `id` | record | |
| `user` | record\<user\> | |
| `year` | int | |
| `target` | int | books to read |
| `completed` | int | maintained by app |

**Index:** `(user, year)` UNIQUE

### `collection`
| Field | Type | Notes |
|---|---|---|
| `id` | record | |
| `user` | record\<user\> | |
| `name` | string | |
| `description` | option\<string\> | |
| `cover_url` | option\<string\> | |
| `is_public` | bool | |
| `created_at` | datetime | READONLY |
| `updated_at` | datetime | auto |

### `collection_book` *(graph relation: collection → book)*
| Field | Type | Notes |
|---|---|---|
| `in` | record\<collection\> | |
| `out` | record\<book\> | |
| `position` | option\<int\> | manual ordering |
| `note` | option\<string\> | |
| `added_at` | datetime | READONLY |

**Index:** `(in, out)` UNIQUE

### `highlight`
| Field | Type | Notes |
|---|---|---|
| `id` | record | |
| `user` | record\<user\> | |
| `book` | record\<book\> | |
| `chapter` | record\<chapter\> | |
| `offset_start` | int | char position |
| `offset_end` | int | char position |
| `paragraph` | int | resilience fallback |
| `text_snapshot` | string | copy of selected text |
| `color` | string | `yellow \| green \| blue \| pink \| purple` |
| `note` | option\<string\> | personal annotation |
| `is_public` | bool | |
| `created_at` | datetime | READONLY |
| `updated_at` | datetime | auto |

### `comment`
| Field | Type | Notes |
|---|---|---|
| `id` | record | |
| `user` | record\<user\> | |
| `book` | record\<book\> | |
| `chapter` | record\<chapter\> | |
| `highlight` | option\<record\<highlight\>\> | if on a highlight |
| `parent` | option\<record\<comment\>\> | threading |
| `body` | string | |
| `is_spoiler` | bool | |
| `offset_start` | option\<int\> | inline, not on highlight |
| `offset_end` | option\<int\> | |
| `text_snapshot` | option\<string\> | |
| `deleted_at` | option\<datetime\> | soft delete |
| `created_at` | datetime | READONLY |
| `updated_at` | datetime | auto |

### `comment_vote` *(graph relation: user → comment)*
| Field | Type | Notes |
|---|---|---|
| `in` | record\<user\> | |
| `out` | record\<comment\> | |
| `value` | int | `1` or `-1` |
| `created_at` | datetime | READONLY |

### `book_review` *(graph relation: user → book)*
| Field | Type | Notes |
|---|---|---|
| `in` | record\<user\> | |
| `out` | record\<book\> | |
| `rating` | int | 1–5, asserted |
| `title` | option\<string\> | |
| `body` | option\<string\> | |
| `contains_spoiler` | bool | |
| `reading_status` | string | `reading \| completed \| dropped \| unread` |
| `verified_reader` | bool | has bookmark at write time |
| `helpful_count` | int | denormalized, kept by event |
| `status` | string | `draft \| published \| flagged \| removed` |
| `created_at` | datetime | READONLY |
| `updated_at` | datetime | auto |

**Events:** `book_review_stats` → updates `book.avg_rating`, `book.review_count`

### `chapter_review` *(graph relation: user → chapter)*
| Field | Type | Notes |
|---|---|---|
| `in` | record\<user\> | |
| `out` | record\<chapter\> | |
| `rating` | int | 1–5, asserted |
| `body` | option\<string\> | |
| `contains_spoiler` | bool | |
| `helpful_count` | int | |
| `status` | string | `draft \| published \| flagged \| removed` |
| `created_at` | datetime | READONLY |
| `updated_at` | datetime | auto |

**Events:** `chapter_review_stats` → updates `chapter.avg_rating`, `chapter.review_count`

### `book_review_vote` / `chapter_review_vote`
| Field | Type | Notes |
|---|---|---|
| `in` | record\<user\> | |
| `out` | record\<book_review\> or record\<chapter_review\> | |
| `value` | int | `1` or `-1` |
| `created_at` | datetime | READONLY |

### `review_flag` *(polymorphic)*
| Field | Type | Notes |
|---|---|---|
| `id` | record | |
| `flagged_by` | record\<user\> | |
| `review` | record | `book_review:x` or `chapter_review:x` |
| `reason` | string | `spoiler \| spam \| offensive \| inaccurate \| other` |
| `note` | option\<string\> | |
| `resolved` | bool | |
| `resolved_by` | option\<record\<user\>\> | |
| `created_at` | datetime | READONLY |

### `word_translation`
| Field | Type | Notes |
|---|---|---|
| `id` | record | |
| `word` | string | original word/phrase |
| `translation` | string | proposed translation |
| `source_lang` | string | e.g. `"ur"` |
| `target_lang` | string | e.g. `"en"` |
| `submitted_by` | record\<user\> | |
| `scope` | string | `global \| book \| chapter` |
| `book` | option\<record\<book\>\> | set if scope = book |
| `chapter` | option\<record\<chapter\>\> | set if scope = chapter |
| `context_note` | option\<string\> | |
| `upvotes` | int | maintained by event |
| `downvotes` | int | maintained by event |
| `created_at` | datetime | READONLY |
| `updated_at` | datetime | auto |

**Scope resolution priority:** chapter → book → global

### `translation_vote` *(graph relation: user → word_translation)*
| Field | Type | Notes |
|---|---|---|
| `in` | record\<user\> | |
| `out` | record\<word_translation\> | |
| `value` | int | `1` or `-1` |
| `created_at` | datetime | READONLY |
