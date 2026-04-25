use async_graphql::{Context, InputObject, Object, Result, SimpleObject};
use chrono::Datelike;

use crate::api::middleware::Claims;
use crate::db::bookmark_repo::{BookmarkResponse, ReadingGoalResponse, UpsertBookmarkDto};
use crate::db::collection_repo::{
    AddBookDto, CollectionBookResponse, CollectionResponse, CreateCollectionDto,
    UpdateCollectionDto,
};
use crate::state::AppState;

// ── GQL output types ──────────────────────────────────────────────────────────

#[derive(SimpleObject)]
pub struct CollectionGql {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub is_public: bool,
}

impl From<CollectionResponse> for CollectionGql {
    fn from(r: CollectionResponse) -> Self {
        CollectionGql {
            id: r.id,
            user_id: r.user_id,
            name: r.name,
            description: r.description,
            cover_url: r.cover_url,
            is_public: r.is_public,
        }
    }
}

#[derive(SimpleObject)]
pub struct CollectionBookGql {
    pub book_id: String,
    pub position: Option<i64>,
    pub note: Option<String>,
}

impl From<CollectionBookResponse> for CollectionBookGql {
    fn from(r: CollectionBookResponse) -> Self {
        CollectionBookGql {
            book_id: r.book_id,
            position: r.position,
            note: r.note,
        }
    }
}

#[derive(SimpleObject)]
pub struct BookmarkGql {
    pub id: String,
    pub book_id: String,
    pub status: String,
    pub progress: Option<i64>,
    pub notes: Option<String>,
}

impl From<BookmarkResponse> for BookmarkGql {
    fn from(r: BookmarkResponse) -> Self {
        BookmarkGql {
            id: r.id,
            book_id: r.book_id,
            status: r.status,
            progress: r.progress,
            notes: r.notes,
        }
    }
}

#[derive(SimpleObject)]
pub struct ReadingGoalGql {
    pub id: String,
    pub year: i64,
    pub target: i64,
    pub completed: i64,
    pub progress_pct: f64,
}

impl From<ReadingGoalResponse> for ReadingGoalGql {
    fn from(r: ReadingGoalResponse) -> Self {
        ReadingGoalGql {
            id: r.id,
            year: r.year,
            target: r.target,
            completed: r.completed,
            progress_pct: r.progress_pct,
        }
    }
}

// ── GQL input types ───────────────────────────────────────────────────────────

#[derive(InputObject)]
pub struct CreateCollectionInput {
    pub name: String,
    pub description: Option<String>,
    pub is_public: Option<bool>,
}

#[derive(InputObject)]
pub struct UpdateCollectionInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
}

#[derive(InputObject)]
pub struct AddBookInput {
    pub book_slug: String,
    pub position: Option<i64>,
    pub note: Option<String>,
}

#[derive(InputObject)]
pub struct UpsertBookmarkInput {
    pub status: String,
    pub progress: Option<i64>,
    pub notes: Option<String>,
}

// ── Query ─────────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct CollectionQuery;

#[Object]
impl CollectionQuery {
    async fn my_collections(
        &self,
        ctx: &Context<'_>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<CollectionGql>> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        let collections = state
            .services
            .collection_repo
            .list_user_collections(&claims.sub, limit.unwrap_or(20), offset.unwrap_or(0))
            .await?;
        Ok(collections.into_iter().map(Into::into).collect())
    }

    async fn collection(&self, ctx: &Context<'_>, id: String) -> Result<Option<CollectionGql>> {
        let state = ctx.data::<AppState>()?;
        let collection = state.services.collection_repo.get_collection(&id).await?;
        Ok(collection.map(Into::into))
    }

    async fn collection_books(
        &self,
        ctx: &Context<'_>,
        collection_id: String,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<CollectionBookGql>> {
        let _claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        let books = state
            .services
            .collection_repo
            .list_collection_books(&collection_id, limit.unwrap_or(20), offset.unwrap_or(0))
            .await?;
        Ok(books.into_iter().map(Into::into).collect())
    }

    async fn my_bookmarks(
        &self,
        ctx: &Context<'_>,
        status: Option<String>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<BookmarkGql>> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        let bookmarks = state
            .services
            .bookmark_repo
            .list_user_bookmarks(
                &claims.sub,
                status.as_deref(),
                limit.unwrap_or(20),
                offset.unwrap_or(0),
            )
            .await?;
        Ok(bookmarks.into_iter().map(Into::into).collect())
    }

    async fn my_reading_goal(
        &self,
        ctx: &Context<'_>,
        year: Option<i64>,
    ) -> Result<Option<ReadingGoalGql>> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        let effective_year = year.unwrap_or_else(|| chrono::Utc::now().year() as i64);
        let goal = state
            .services
            .bookmark_repo
            .get_reading_goal(&claims.sub, effective_year)
            .await?;
        Ok(goal.map(Into::into))
    }
}

// ── Mutation ──────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct CollectionMutation;

#[Object]
impl CollectionMutation {
    async fn create_collection(
        &self,
        ctx: &Context<'_>,
        input: CreateCollectionInput,
    ) -> Result<CollectionGql> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;

        let dto = CreateCollectionDto {
            name: input.name,
            description: input.description,
            is_public: input.is_public,
        };

        let collection = state
            .services
            .collection_repo
            .create_collection(&claims.sub, dto)
            .await?;
        Ok(collection.into())
    }

    async fn update_collection(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateCollectionInput,
    ) -> Result<Option<CollectionGql>> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;

        let dto = UpdateCollectionDto {
            name: input.name,
            description: input.description,
            is_public: input.is_public,
        };

        let collection = state
            .services
            .collection_repo
            .update_collection(&id, &claims.sub, dto)
            .await?;
        Ok(collection.map(Into::into))
    }

    async fn delete_collection(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state
            .services
            .collection_repo
            .delete_collection(&id, &claims.sub)
            .await?;
        Ok(true)
    }

    async fn add_book_to_collection(
        &self,
        ctx: &Context<'_>,
        collection_id: String,
        input: AddBookInput,
    ) -> Result<CollectionBookGql> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;

        let dto = AddBookDto {
            position: input.position,
            note: input.note,
        };

        let entry = state
            .services
            .collection_repo
            .add_book(&collection_id, &input.book_slug, &claims.sub, dto)
            .await?;
        Ok(entry.into())
    }

    async fn remove_book_from_collection(
        &self,
        ctx: &Context<'_>,
        collection_id: String,
        book_slug: String,
    ) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state
            .services
            .collection_repo
            .remove_book(&collection_id, &book_slug, &claims.sub)
            .await?;
        Ok(true)
    }

    async fn upsert_bookmark(
        &self,
        ctx: &Context<'_>,
        book_slug: String,
        input: UpsertBookmarkInput,
    ) -> Result<BookmarkGql> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;

        let dto = UpsertBookmarkDto {
            status: input.status,
            progress: input.progress,
            notes: input.notes,
        };

        let bookmark = state
            .services
            .bookmark_repo
            .upsert_bookmark(&claims.sub, &book_slug, dto)
            .await?;
        Ok(bookmark.into())
    }

    async fn remove_bookmark(&self, ctx: &Context<'_>, book_slug: String) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state
            .services
            .bookmark_repo
            .remove_bookmark(&claims.sub, &book_slug)
            .await?;
        Ok(true)
    }

    async fn upsert_reading_goal(
        &self,
        ctx: &Context<'_>,
        year: i64,
        target: i64,
    ) -> Result<ReadingGoalGql> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        let goal = state
            .services
            .bookmark_repo
            .upsert_reading_goal(&claims.sub, year, target)
            .await?;
        Ok(goal.into())
    }
}
