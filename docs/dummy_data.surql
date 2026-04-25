-- =============================================================================
-- Dummy Data Seed Script for Local Testing
-- This script populates the database with initial dummy data for development.
-- =============================================================================

-- Define the missing analyzer before creating records that use it
DEFINE ANALYZER IF NOT EXISTS blank TOKENIZERS blank FILTERS lowercase;

-- Users
CREATE user:user1 CONTENT {
    username: "john_doe",
    email: "john@example.com",
    password_hash: "$argon2id$v=19$m=19456,t=2,p=1$w2eA6lM06yW5hQk82fL8rQ$2N5P1c+6lM06yW5hQk82fL8rQ",
    is_active: true,
    is_verified: true
};

CREATE user:user2 CONTENT {
    username: "jane_smith",
    email: "jane@example.com",
    password_hash: "$argon2id$v=19$m=19456,t=2,p=1$w2eA6lM06yW5hQk82fL8rQ$2N5P1c+6lM06yW5hQk82fL8rQ",
    is_active: true,
    is_verified: true
};

CREATE user:admin1 CONTENT {
    username: "admin_user",
    email: "admin@example.com",
    password_hash: "$argon2id$v=19$m=19456,t=2,p=1$w2eA6lM06yW5hQk82fL8rQ$2N5P1c+6lM06yW5hQk82fL8rQ",
    is_active: true,
    is_verified: true
};

-- Profiles
CREATE profile:user1 CONTENT {
    user: user:user1,
    first_name: "John",
    last_name: "Doe",
    display_name: "John D.",
    bio: "Avid reader and sci-fi enthusiast."
};

CREATE profile:user2 CONTENT {
    user: user:user2,
    first_name: "Jane",
    last_name: "Smith",
    display_name: "Jane S.",
    bio: "Fantasy lover."
};

-- Assign roles
RELATE user:admin1->assigned_role->role:admin;
RELATE user:user1->assigned_role->role:user;
RELATE user:user2->assigned_role->role:user;

-- Publishers
CREATE publisher:pub1 CONTENT {
    name: "Penguin Random House",
    website: "https://penguin.com",
    country: "USA"
};

CREATE publisher:pub2 CONTENT {
    name: "Tor Books",
    website: "https://tor.com",
    country: "USA"
};

-- Authors
CREATE author:auth1 CONTENT {
    name: "Brandon Sanderson",
    slug: "brandon-sanderson",
    bio: "American author of epic fantasy and science fiction.",
    website: "https://brandonsanderson.com"
};

CREATE author:auth2 CONTENT {
    name: "Frank Herbert",
    slug: "frank-herbert",
    bio: "American science fiction author best known for Dune."
};

-- Categories
CREATE category:cat1 CONTENT {
    name: "Fiction",
    slug: "fiction"
};

CREATE category:cat2 CONTENT {
    name: "Science Fiction",
    slug: "science-fiction",
    parent: category:cat1
};

CREATE category:cat3 CONTENT {
    name: "Fantasy",
    slug: "fantasy",
    parent: category:cat1
};

-- Tags
CREATE tag:tag1 CONTENT {
    name: "Epic",
    slug: "epic"
};

CREATE tag:tag2 CONTENT {
    name: "Space Opera",
    slug: "space-opera"
};

CREATE tag:tag3 CONTENT {
    name: "Magic System",
    slug: "magic-system"
};

-- Books
CREATE book:book1 CONTENT {
    title: "The Way of Kings",
    slug: "the-way-of-kings",
    isbn: "9780765326355",
    summary: "The first book of The Stormlight Archive.",
    description: "Roshar is a world of stone and storms...",
    page_count: 1007,
    language: "en",
    publisher: publisher:pub2,
    categories: [category:cat3],
    tags: [tag:tag1, tag:tag3],
    is_published: true
};

CREATE book:book2 CONTENT {
    title: "Dune",
    slug: "dune",
    isbn: "9780441172719",
    summary: "A sweeping epic of a desert planet.",
    description: "Set on the desert planet Arrakis, Dune is the story of the boy Paul Atreides...",
    page_count: 412,
    language: "en",
    publisher: publisher:pub1,
    categories: [category:cat2],
    tags: [tag:tag1, tag:tag2],
    is_published: true
};

-- Wrote Relations
RELATE author:auth1->wrote->book:book1 CONTENT { role: "author" };
RELATE author:auth2->wrote->book:book2 CONTENT { role: "author" };

-- Chapters
CREATE chapter:chap1_1 CONTENT {
    book: book:book1,
    number: 1,
    title: "Stormblessed",
    slug: "stormblessed",
    content: "Kaladin Stormblessed lead his squad into battle...",
    content_format: "markdown",
    is_published: true,
    word_count: 3500,
    reading_time_mins: 15
};

CREATE chapter:chap1_2 CONTENT {
    book: book:book1,
    number: 2,
    title: "Honor is Dead",
    slug: "honor-is-dead",
    content: "Dalinar looked up at the sky and saw the highstorm coming...",
    content_format: "markdown",
    is_published: true,
    word_count: 4000,
    reading_time_mins: 17
};

CREATE chapter:chap2_1 CONTENT {
    book: book:book2,
    number: 1,
    title: "Arrival",
    slug: "arrival",
    content: "The Atreides family arrived on Arrakis, knowing it was a trap...",
    content_format: "markdown",
    is_published: true,
    word_count: 2800,
    reading_time_mins: 12
};

-- Bookmarks
RELATE user:user1->bookmark->book:book1 CONTENT {
    status: "reading",
    progress: 50
};

RELATE user:user2->bookmark->book:book2 CONTENT {
    status: "completed"
};

-- Reviews
RELATE user:user2->book_review->book:book2 CONTENT {
    rating: 5,
    title: "Masterpiece",
    body: "One of the best sci-fi books ever written. The worldbuilding is incredible.",
    reading_status: "completed",
    verified_reader: true,
    status: "published"
};
