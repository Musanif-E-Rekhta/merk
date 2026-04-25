use async_graphql::{Context, InputObject, Object, Result, SimpleObject};

use crate::api::middleware::Claims;
use crate::db::translation_repo::{CreateTranslationDto, WordTranslationResponse};
use crate::state::AppState;

#[derive(SimpleObject)]
pub struct WordTranslationGql {
    pub id: String,
    pub word: String,
    pub translation: String,
    pub source_lang: String,
    pub target_lang: String,
    pub submitted_by: String,
    pub scope: String,
    pub book_id: Option<String>,
    pub chapter_id: Option<String>,
    pub context_note: Option<String>,
    pub upvotes: i64,
    pub downvotes: i64,
    pub score: i64,
}

impl From<WordTranslationResponse> for WordTranslationGql {
    fn from(r: WordTranslationResponse) -> Self {
        WordTranslationGql {
            id: r.id,
            word: r.word,
            translation: r.translation,
            source_lang: r.source_lang,
            target_lang: r.target_lang,
            submitted_by: r.submitted_by,
            scope: r.scope,
            book_id: r.book_id,
            chapter_id: r.chapter_id,
            context_note: r.context_note,
            upvotes: r.upvotes,
            downvotes: r.downvotes,
            score: r.score,
        }
    }
}

#[derive(InputObject)]
pub struct CreateTranslationInput {
    pub word: String,
    pub translation: String,
    pub source_lang: String,
    pub target_lang: String,
    pub scope: String,
    pub book_slug: Option<String>,
    pub chapter_slug: Option<String>,
    pub context_note: Option<String>,
}

#[derive(Default)]
pub struct TranslationQuery;

#[Object]
impl TranslationQuery {
    /// Look up translations for a word, priority-ordered: chapter → book → global.
    async fn word_translations(
        &self,
        ctx: &Context<'_>,
        word: String,
        target_lang: String,
        book_slug: Option<String>,
        chapter_slug: Option<String>,
    ) -> Result<Vec<WordTranslationGql>> {
        let state = ctx.data::<AppState>()?;
        let translations = state
            .services
            .translation_repo
            .get_word_translations(
                &word,
                &target_lang,
                book_slug.as_deref(),
                chapter_slug.as_deref(),
            )
            .await?;
        Ok(translations.into_iter().map(Into::into).collect())
    }
}

#[derive(Default)]
pub struct TranslationMutation;

#[Object]
impl TranslationMutation {
    async fn submit_translation(
        &self,
        ctx: &Context<'_>,
        input: CreateTranslationInput,
    ) -> Result<WordTranslationGql> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        let dto = CreateTranslationDto {
            word: input.word,
            translation: input.translation,
            source_lang: input.source_lang,
            target_lang: input.target_lang,
            scope: input.scope,
            book_slug: input.book_slug,
            chapter_slug: input.chapter_slug,
            context_note: input.context_note,
        };
        let translation = state
            .services
            .translation_repo
            .create_translation(&claims.sub, dto)
            .await?;
        Ok(translation.into())
    }

    async fn vote_translation(
        &self,
        ctx: &Context<'_>,
        translation_id: String,
        value: i64,
    ) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state
            .services
            .translation_repo
            .vote_translation(&claims.sub, &translation_id, value)
            .await?;
        Ok(true)
    }
}
