//! Smoke tests for the authentication flow.
//!
//! Uses an in-memory SurrealDB (`mem://`) so the suite stays hermetic —
//! no docker-compose required. Each test starts with a fresh DB,
//! migrates it, and exercises a single flow.

use async_trait::async_trait;
use merk::config::AppConfig;
use merk::db::refresh_token_repo::RefreshTokenRepo;
use merk::services::mailer::{Mailer, MailerError, NoopMailer};
use merk::services::user_service::{LoginOutcome, UserService};
use std::sync::{Arc, Mutex};
use surrealdb::engine::any::connect;

#[derive(Default)]
struct CapturingMailer {
    sent: Mutex<Vec<(String, String)>>,
}

#[async_trait]
impl Mailer for CapturingMailer {
    async fn send_password_reset(
        &self,
        to_email: &str,
        reset_url: &str,
    ) -> Result<(), MailerError> {
        self.sent
            .lock()
            .unwrap()
            .push((to_email.to_string(), reset_url.to_string()));
        Ok(())
    }

    async fn send_email_verification(
        &self,
        _to_email: &str,
        _verify_url: &str,
    ) -> Result<(), MailerError> {
        Ok(())
    }
}

async fn fresh_db_and_service() -> (UserService, Arc<AppConfig>) {
    let db = connect("mem://").await.unwrap();
    db.use_ns("test").use_db("test").await.unwrap();
    merk_migrations::Migrator::up(&db, None).await.unwrap();

    let config = Arc::new(AppConfig {
        host: "127.0.0.1".into(),
        port: None,
        enable_tls: false,
        tls_alt_name: String::new(),
        surrealdb_url: "mem://".into(),
        surrealdb_user: String::new(),
        surrealdb_pass: String::new(),
        surrealdb_ns: "test".into(),
        surrealdb_db: "test".into(),
        jwt_secret: "test-secret-32-bytes-or-longer-1234567890".into(),
        mail_transport: "noop".into(),
        mail_reset_url_template: "http://localhost/reset?token={token}".into(),
        mail_verify_url_template: "http://localhost/verify?token={token}".into(),
        cors_origins: String::new(),
        mail_smtp_host: String::new(),
        mail_smtp_port: 587,
        mail_smtp_user: String::new(),
        mail_smtp_pass: String::new(),
        mail_smtp_from: "Musanif <noreply@musanif.test>".into(),
    });

    let user_repo = merk::db::user_repo::UserRepo::new(db.clone());
    let refresh_repo = RefreshTokenRepo::new(db.clone());
    let service = UserService::new(
        user_repo,
        refresh_repo,
        config.clone(),
        Arc::new(NoopMailer),
    );
    (service, config)
}

#[tokio::test]
async fn register_then_login_issues_access_token() {
    let (svc, _config) = fresh_db_and_service().await;

    let (tokens, user) = svc
        .register("alice".into(), "alice@test".into(), "Password!23".into())
        .await
        .expect("register failed");

    assert!(!tokens.access.is_empty(), "access token should be issued");
    assert!(
        !tokens.refresh.is_empty(),
        "refresh token should be issued"
    );
    assert_eq!(user.username, "alice");

    let outcome = svc
        .login("alice@test".into(), "Password!23".into())
        .await
        .expect("login failed");

    match outcome {
        LoginOutcome::Authenticated { tokens, .. } => {
            assert!(!tokens.access.is_empty());
            assert!(!tokens.refresh.is_empty());
        }
        LoginOutcome::Requires2fa { .. } => panic!("2FA should not be required for fresh user"),
    }
}

#[tokio::test]
async fn login_rejects_wrong_password() {
    let (svc, _config) = fresh_db_and_service().await;

    svc.register("bob".into(), "bob@test".into(), "Password!23".into())
        .await
        .unwrap();

    let result = svc.login("bob@test".into(), "wrong".into()).await;
    assert!(result.is_err(), "wrong password must fail");
}

#[tokio::test]
async fn login_rejects_unknown_email() {
    let (svc, _config) = fresh_db_and_service().await;
    let result = svc.login("nobody@test".into(), "Password!23".into()).await;
    assert!(result.is_err(), "unknown user must fail");
}

#[tokio::test]
async fn change_password_invalidates_old_credentials() {
    let (svc, _config) = fresh_db_and_service().await;

    let (_tokens, user) = svc
        .register("carol".into(), "carol@test".into(), "OldPassword!23".into())
        .await
        .unwrap();

    svc.change_password(&user.id, "OldPassword!23", "NewPassword!23")
        .await
        .expect("change_password failed");

    // Old password rejected
    assert!(
        svc.login("carol@test".into(), "OldPassword!23".into())
            .await
            .is_err()
    );
    // New password works
    assert!(
        svc.login("carol@test".into(), "NewPassword!23".into())
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn forgot_password_dispatches_mail_with_reset_url() {
    let db = connect("mem://").await.unwrap();
    db.use_ns("test").use_db("test").await.unwrap();
    merk_migrations::Migrator::up(&db, None).await.unwrap();

    let config = Arc::new(AppConfig {
        host: "127.0.0.1".into(),
        port: None,
        enable_tls: false,
        tls_alt_name: String::new(),
        surrealdb_url: "mem://".into(),
        surrealdb_user: String::new(),
        surrealdb_pass: String::new(),
        surrealdb_ns: "test".into(),
        surrealdb_db: "test".into(),
        jwt_secret: "test-secret-32-bytes-or-longer-1234567890".into(),
        mail_transport: "noop".into(),
        mail_reset_url_template: "https://app.test/reset?token={token}".into(),
        mail_verify_url_template: "https://app.test/verify?token={token}".into(),
        cors_origins: String::new(),
        mail_smtp_host: String::new(),
        mail_smtp_port: 587,
        mail_smtp_user: String::new(),
        mail_smtp_pass: String::new(),
        mail_smtp_from: "Musanif <noreply@musanif.test>".into(),
    });
    let mailer = Arc::new(CapturingMailer::default());
    let user_repo = merk::db::user_repo::UserRepo::new(db.clone());
    let refresh_repo = RefreshTokenRepo::new(db.clone());
    let svc = UserService::new(user_repo, refresh_repo, config.clone(), mailer.clone());

    svc.register("dana".into(), "dana@test".into(), "Password!23".into())
        .await
        .unwrap();

    svc.forgot_password("dana@test".into()).await.unwrap();

    let sent = mailer.sent.lock().unwrap();
    assert_eq!(sent.len(), 1, "exactly one mail should be dispatched");
    let (to, url) = &sent[0];
    assert_eq!(to, "dana@test");
    assert!(url.starts_with("https://app.test/reset?token="));
    assert!(!url.ends_with("token=") && !url.ends_with("{token}"));
}

#[tokio::test]
async fn refresh_token_rotates_pair_and_revokes_old() {
    let (svc, _config) = fresh_db_and_service().await;

    let (initial, _) = svc
        .register("eve".into(), "eve@test".into(), "Password!23".into())
        .await
        .unwrap();

    let (rotated, user) = svc
        .refresh_session(initial.refresh.clone())
        .await
        .expect("first refresh should succeed");
    assert_eq!(user.username, "eve");
    // Access JWTs are deterministic per (user, exp-second) so we can't
    // require rotation — `exp` only ticks once a second. The refresh
    // token MUST rotate, since it carries 256 bits of entropy.
    assert!(!rotated.access.is_empty(), "access JWT must be issued");
    assert_ne!(rotated.refresh, initial.refresh, "refresh token should rotate");

    // Reusing the consumed refresh token must fail.
    let reuse = svc.refresh_session(initial.refresh).await;
    assert!(reuse.is_err(), "old refresh must not be reusable");

    // The freshly rotated refresh keeps working.
    let (next, _) = svc.refresh_session(rotated.refresh.clone()).await.unwrap();
    assert_ne!(next.refresh, rotated.refresh);
}

#[tokio::test]
async fn logout_revokes_refresh_token() {
    let (svc, _config) = fresh_db_and_service().await;

    let (tokens, _) = svc
        .register("frank".into(), "frank@test".into(), "Password!23".into())
        .await
        .unwrap();

    svc.logout(tokens.refresh.clone()).await.unwrap();

    let result = svc.refresh_session(tokens.refresh).await;
    assert!(result.is_err(), "revoked refresh token must not refresh");
}
