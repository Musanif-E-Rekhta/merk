pub mod event_bus;
pub mod mailer;
pub mod pipeline;
pub mod rbac;
pub mod user_service;

use crate::config::AppConfig;
use crate::db::Db;
use crate::db::admin::ai::AiRepo;
use crate::db::admin::covers::CoversRepo;
use crate::db::admin::drafts::DraftsRepo;
use crate::db::admin::jobs::JobsRepo;
use crate::db::admin::publish::PublishRepo;
use crate::db::book_repo::BookRepo;
use crate::db::bookmark_repo::BookmarkRepo;
use crate::db::chapter_repo::ChapterRepo;
use crate::db::collection_repo::CollectionRepo;
use crate::db::comment_repo::CommentRepo;
use crate::db::highlight_repo::HighlightRepo;
use crate::db::profile_repo::ProfileRepo;
use crate::db::rbac_repo::RbacRepo;
use crate::db::refresh_token_repo::RefreshTokenRepo;
use crate::db::review_repo::ReviewRepo;
use crate::db::translation_repo::TranslationRepo;
use crate::db::user_repo::UserRepo;
use crate::services::event_bus::EventBus;
use crate::services::mailer::Mailer;
use crate::services::rbac::RbacService;
use crate::services::user_service::UserService;
use merk_blob_store::{BlobStore, LocalBlobStore};
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
    pub rbac: RbacService,
    pub admin_jobs: JobsRepo,
    pub admin_drafts: DraftsRepo,
    pub admin_ai: AiRepo,
    pub admin_covers: CoversRepo,
    pub admin_publish: PublishRepo,
    pub event_bus: Arc<EventBus>,
    pub blob_store: Arc<dyn BlobStore>,
    pub mailer: Arc<dyn Mailer>,
}

impl Services {
    pub fn new(db: Db, config: Arc<AppConfig>) -> Self {
        let event_bus = EventBus::new();
        let blob_store: Arc<dyn BlobStore> = Arc::new(LocalBlobStore::new("./storage"));
        let mailer = mailer::build(&config);
        let refresh_repo = RefreshTokenRepo::new(db.clone());
        Self {
            user: UserService::new(
                UserRepo::new(db.clone()),
                refresh_repo,
                config,
                mailer.clone(),
            ),
            book_repo: BookRepo::new(db.clone()),
            profile_repo: ProfileRepo::new(db.clone()),
            chapter_repo: ChapterRepo::new(db.clone()),
            comment_repo: CommentRepo::new(db.clone()),
            highlight_repo: HighlightRepo::new(db.clone()),
            bookmark_repo: BookmarkRepo::new(db.clone()),
            collection_repo: CollectionRepo::new(db.clone()),
            review_repo: ReviewRepo::new(db.clone()),
            translation_repo: TranslationRepo::new(db.clone()),
            rbac_repo: RbacRepo::new(db.clone()),
            rbac: RbacService::new(db.clone()),
            admin_jobs: JobsRepo::new(db.clone()),
            admin_drafts: DraftsRepo::new(db.clone()),
            admin_ai: AiRepo::new(db.clone()),
            admin_covers: CoversRepo::new(db.clone()),
            admin_publish: PublishRepo::new(db),
            event_bus,
            blob_store,
            mailer,
        }
    }
}
