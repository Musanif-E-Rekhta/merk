pub mod auth;
pub mod user_service;

use crate::db::Db;
use crate::db::book_repo::BookRepo;
use crate::db::bookmark_repo::BookmarkRepo;
use crate::db::chapter_repo::ChapterRepo;
use crate::db::collection_repo::CollectionRepo;
use crate::db::comment_repo::CommentRepo;
use crate::db::highlight_repo::HighlightRepo;
use crate::db::profile_repo::ProfileRepo;
use crate::db::rbac_repo::RbacRepo;
use crate::db::review_repo::ReviewRepo;
use crate::db::translation_repo::TranslationRepo;
use crate::db::user_repo::UserRepo;
use crate::services::user_service::UserService;
use std::sync::Arc;

pub struct Services {
    pub user: UserService,
    pub book_repo: BookRepo,
    pub profile_repo: ProfileRepo,
    pub chapter_repo: ChapterRepo,
    pub comment_repo: CommentRepo,
    pub highlight_repo: HighlightRepo,
    pub bookmark_repo: BookmarkRepo,
    pub collection_repo: CollectionRepo,
    pub review_repo: ReviewRepo,
    pub translation_repo: TranslationRepo,
    pub rbac_repo: RbacRepo,
}

impl Services {
    pub fn new(db: Db, config: Arc<crate::config::AppConfig>) -> Self {
        Self {
            user: UserService::new(UserRepo::new(db.clone()), config),
            book_repo: BookRepo::new(db.clone()),
            profile_repo: ProfileRepo::new(db.clone()),
            chapter_repo: ChapterRepo::new(db.clone()),
            comment_repo: CommentRepo::new(db.clone()),
            highlight_repo: HighlightRepo::new(db.clone()),
            bookmark_repo: BookmarkRepo::new(db.clone()),
            collection_repo: CollectionRepo::new(db.clone()),
            review_repo: ReviewRepo::new(db.clone()),
            translation_repo: TranslationRepo::new(db.clone()),
            rbac_repo: RbacRepo::new(db),
        }
    }
}
