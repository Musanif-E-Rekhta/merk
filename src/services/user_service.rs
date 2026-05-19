use crate::db::book_repo::AuthorResponse;
use crate::db::record_id_key_to_string;
use crate::db::refresh_token_repo::RefreshTokenRepo;
use crate::db::user_repo::{
    CreateUserDto, ReadingSessionResponse, RecordReadingSessionDto, UserRepo, UserResponse,
    UserStats,
};
use crate::error::Error;
use crate::services::mailer::Mailer;
use merk_auth::{
    generate_2fa_challenge, generate_jwt, generate_reset_token, verify_2fa_challenge,
    verify_password,
};
use merk_totp as totp;
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Long-lived (30d) refresh-token lifetime. Coupled to the migration
/// 0020 schema's `expires_at` field — the row is only considered active
/// while `expires_at > now()`.
const REFRESH_TOKEN_TTL_DAYS: i64 = 30;

/// Pair issued at every successful login. The access JWT is used for
/// every authenticated request; the refresh token rotates them.
#[derive(Debug, Clone)]
pub struct AuthTokens {
    pub access: String,
    pub refresh: String,
}

// ── 2FA-aware login outcome ───────────────────────────────────────────────────

/// Result of a password-only login attempt. Either we issue an access
/// token straight away (no 2FA), or the caller must complete a TOTP
/// challenge first.
pub enum LoginOutcome {
    Authenticated {
        tokens: AuthTokens,
        user: UserResponse,
    },
    Requires2fa {
        challenge: String,
        user: UserResponse,
    },
}

#[derive(Debug, Clone)]
pub struct Setup2faResponse {
    /// Plaintext base32 secret; never re-served after setup completes.
    pub secret: String,
    /// `otpauth://totp/...` URL the QR-code renderer scans.
    pub otpauth_url: String,
    /// Inline SVG of the `otpauth_url` rendered as a QR code. The client
    /// drops this straight into the page so the secret never visits a
    /// third-party renderer.
    pub qr_svg: String,
    /// Plaintext recovery codes; only returned once.
    pub recovery_codes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TwoFactorStatus {
    pub enabled: bool,
    pub last_used_at: Option<chrono::DateTime<Utc>>,
}

pub struct UserService {
    repo: UserRepo,
    refresh_repo: RefreshTokenRepo,
    config: Arc<crate::config::AppConfig>,
    mailer: Arc<dyn Mailer>,
}

impl UserService {
    pub fn new(
        repo: UserRepo,
        refresh_repo: RefreshTokenRepo,
        config: Arc<crate::config::AppConfig>,
        mailer: Arc<dyn Mailer>,
    ) -> Self {
        Self {
            repo,
            refresh_repo,
            config,
            mailer,
        }
    }

    fn hash_token(token: &str) -> String {
        let digest = Sha256::digest(token.as_bytes());
        let mut s = String::with_capacity(64);
        for b in digest.iter() {
            use std::fmt::Write;
            let _ = write!(s, "{:02x}", b);
        }
        s
    }

    /// Issue and persist a refresh token for `user_id`. Returns the
    /// plaintext value (only seen here and at the API boundary).
    async fn issue_refresh(&self, user_id: &str) -> Result<String, Error> {
        let plaintext = generate_reset_token();
        let hash = Self::hash_token(&plaintext);
        let expires_at = Utc::now() + Duration::days(REFRESH_TOKEN_TTL_DAYS);
        self.refresh_repo
            .create(user_id, &hash, expires_at)
            .await?;
        Ok(plaintext)
    }

    async fn issue_token_pair(&self, user_id: &str, is_active: bool) -> Result<AuthTokens, Error> {
        let access = generate_jwt(user_id, is_active, &self.config.jwt_secret)?;
        let refresh = self.issue_refresh(user_id).await?;
        Ok(AuthTokens { access, refresh })
    }

    pub async fn register(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<(AuthTokens, UserResponse), Error> {
        let dto = CreateUserDto {
            username,
            email,
            raw_password: password,
        };

        let user = self.repo.create_user(dto).await?;

        let id_str = user
            .id
            .as_ref()
            .ok_or_else(|| Error::internal("auth", "User ID missing after creation"))
            .map(|r| record_id_key_to_string(&r.key))?;

        let tokens = self.issue_token_pair(&id_str, user.is_active).await?;

        Ok((tokens, user.into()))
    }

    pub async fn login(&self, email: String, password: String) -> Result<LoginOutcome, Error> {
        let user = self
            .repo
            .get_user_by_email(&email)
            .await
            .map_err(|_| Error::wrong_credentials())?
            .ok_or_else(Error::wrong_credentials)?;

        if !verify_password(&password, &user.password_hash) {
            return Err(Error::wrong_credentials());
        }

        if !user.is_active {
            return Err(Error::forbidden("banned_user", "Account is suspended"));
        }

        let id_str = user
            .id
            .as_ref()
            .ok_or_else(|| Error::internal("auth", "User ID missing"))
            .map(|r| record_id_key_to_string(&r.key))?;

        // 2FA gate. `totp_enabled_at` is `Some` only after a successful
        // `Verify2fa` — mid-setup users still authenticate without a code.
        if user.totp_enabled_at.is_some() {
            let challenge = generate_2fa_challenge(&id_str, &self.config.jwt_secret)?;
            return Ok(LoginOutcome::Requires2fa {
                challenge,
                user: user.into(),
            });
        }

        let _ = self.repo.update_last_login(&id_str).await;
        let tokens = self.issue_token_pair(&id_str, user.is_active).await?;
        Ok(LoginOutcome::Authenticated {
            tokens,
            user: user.into(),
        })
    }

    /// Second leg of 2FA login. Accepts either a 6-digit TOTP code or a
    /// recovery code; recovery codes are consumed on successful match.
    pub async fn complete_2fa_login(
        &self,
        challenge: String,
        code: String,
    ) -> Result<(AuthTokens, UserResponse), Error> {
        let user_id = verify_2fa_challenge(&challenge, &self.config.jwt_secret)?;
        let user = self
            .repo
            .get_user_by_id(&user_id)
            .await?
            .ok_or_else(Error::wrong_credentials)?;

        // Must still be 2FA-enabled. If the user disabled 2FA in another
        // session between the password leg and the code leg, refuse the
        // challenge rather than silently issue a token.
        if user.totp_enabled_at.is_none() {
            return Err(Error::bad_request(
                "2fa_disabled",
                "2FA is no longer enabled on this account",
            ));
        }

        let secret_blob = user
            .totp_secret_enc
            .as_deref()
            .ok_or_else(|| Error::internal("auth", "totp_enabled but no secret"))?;
        let secret = totp::decrypt_secret(secret_blob, &self.config.jwt_secret)?;

        let id_str = user
            .id
            .as_ref()
            .ok_or_else(|| Error::internal("auth", "User ID missing"))
            .map(|r| record_id_key_to_string(&r.key))?;

        // Try TOTP first; fall back to recovery code if present.
        let totp_ok = totp::verify_code(&secret, &code)?;
        if totp_ok {
            let _ = self.repo.mark_totp_used(&id_str).await;
        } else {
            let hashes = user.totp_recovery_codes.clone().unwrap_or_default();
            match totp::match_recovery_code(&code, &hashes) {
                Some(idx) => {
                    self.repo.consume_recovery_code(&id_str, idx).await?;
                }
                None => return Err(Error::wrong_credentials()),
            }
        }

        let _ = self.repo.update_last_login(&id_str).await;
        let tokens = self.issue_token_pair(&id_str, user.is_active).await?;
        Ok((tokens, user.into()))
    }

    /// Exchange a refresh token for a new access JWT + a rotated refresh
    /// token. The submitted refresh token is revoked atomically — a
    /// stolen one cannot be reused after a legitimate rotation.
    pub async fn refresh_session(
        &self,
        refresh_token: String,
    ) -> Result<(AuthTokens, UserResponse), Error> {
        let hash = Self::hash_token(&refresh_token);
        let row = self
            .refresh_repo
            .find_active_by_hash(&hash)
            .await?
            .ok_or_else(|| Error::wrong_credentials())?;

        let user_id = row.user_id_str();
        let user = self
            .repo
            .get_user_by_id(&user_id)
            .await?
            .ok_or_else(|| Error::wrong_credentials())?;

        if !user.is_active {
            return Err(Error::forbidden("banned_user", "Account is suspended"));
        }

        // Revoke the consumed token before issuing the new pair so a
        // crash mid-rotation doesn't leave both tokens active.
        if let Some(id) = row.id_str() {
            self.refresh_repo.revoke(&id).await?;
        }

        let tokens = self.issue_token_pair(&user_id, user.is_active).await?;
        Ok((tokens, user.into()))
    }

    /// Mark the supplied refresh token revoked. Idempotent — unknown or
    /// already-revoked tokens succeed silently to avoid signalling
    /// validity to a caller who shouldn't have it.
    pub async fn logout(&self, refresh_token: String) -> Result<(), Error> {
        let hash = Self::hash_token(&refresh_token);
        let _ = self.refresh_repo.revoke_by_hash(&hash).await?;
        Ok(())
    }

    // ── 2FA setup / verify / disable ────────────────────────────────────────

    /// Begin 2FA setup: generate a fresh secret + recovery codes, persist
    /// the encrypted secret, and return everything the client needs to
    /// render the QR code. The user is **not** 2FA-enabled until they
    /// complete `verify_2fa`.
    pub async fn setup_2fa(&self, user_id: &str) -> Result<Setup2faResponse, Error> {
        let user = self
            .repo
            .get_user_by_id(user_id)
            .await?
            .ok_or_else(|| Error::not_found("User not found"))?;

        if user.totp_enabled_at.is_some() {
            return Err(Error::bad_request(
                "2fa_already_enabled",
                "Disable 2FA before generating a new secret",
            ));
        }

        let secret = totp::generate_secret();
        let encrypted = totp::encrypt_secret(&secret, &self.config.jwt_secret)?;
        let url = totp::otpauth_url(&secret, "Musanif", &user.email)?;
        let qr_svg = totp::otpauth_qr_svg(&secret, "Musanif", &user.email)?;
        let recovery_codes = totp::generate_recovery_codes();

        self.repo.set_totp_pending(user_id, &encrypted).await?;

        // Recovery codes are returned now (plaintext, once) and persisted
        // only after `verify_2fa` succeeds — so a setup that's never
        // confirmed leaves no usable codes behind.
        Ok(Setup2faResponse {
            secret,
            otpauth_url: url,
            qr_svg,
            recovery_codes,
        })
    }

    /// Confirm setup by verifying a code against the pending secret. On
    /// success, persist Argon2id-hashed recovery codes and flip
    /// `totp_enabled_at`. The plaintext recovery codes the *caller* is
    /// holding (from `setup_2fa`) become the user's only copies.
    pub async fn verify_2fa(
        &self,
        user_id: &str,
        code: &str,
        plaintext_recovery_codes: &[String],
    ) -> Result<(), Error> {
        let user = self
            .repo
            .get_user_by_id(user_id)
            .await?
            .ok_or_else(|| Error::not_found("User not found"))?;

        let blob = user
            .totp_secret_enc
            .as_deref()
            .ok_or_else(|| Error::bad_request("no_pending_setup", "Run setup_2fa first"))?;
        let secret = totp::decrypt_secret(blob, &self.config.jwt_secret)?;

        if !totp::verify_code(&secret, code)? {
            return Err(Error::bad_request("invalid_code", "Code did not match"));
        }

        let hashes: Vec<String> = plaintext_recovery_codes
            .iter()
            .map(|c| totp::hash_recovery_code(c))
            .collect();
        self.repo.enable_totp(user_id, hashes).await?;
        Ok(())
    }

    /// Disable 2FA. Requires either a current TOTP code or a recovery code,
    /// to prevent a stolen access token from silently downgrading the
    /// account.
    pub async fn disable_2fa(&self, user_id: &str, code: &str) -> Result<(), Error> {
        let user = self
            .repo
            .get_user_by_id(user_id)
            .await?
            .ok_or_else(|| Error::not_found("User not found"))?;

        if user.totp_enabled_at.is_none() {
            // Already off — idempotent, but require a code anyway so we
            // don't wipe a half-set-up secret without proof.
            return Err(Error::bad_request("2fa_not_enabled", "2FA is not enabled"));
        }

        let blob = user
            .totp_secret_enc
            .as_deref()
            .ok_or_else(|| Error::internal("auth", "totp enabled but no secret"))?;
        let secret = totp::decrypt_secret(blob, &self.config.jwt_secret)?;

        let totp_ok = totp::verify_code(&secret, code)?;
        let recovery_idx = if totp_ok {
            None
        } else {
            let hashes = user.totp_recovery_codes.clone().unwrap_or_default();
            match totp::match_recovery_code(code, &hashes) {
                Some(i) => Some(i),
                None => return Err(Error::wrong_credentials()),
            }
        };

        if let Some(i) = recovery_idx {
            // Consume the recovery code even though we're about to clear
            // them — keeps the audit trail honest if disable fails partway.
            self.repo.consume_recovery_code(user_id, i).await?;
        }

        self.repo.disable_totp(user_id).await?;
        Ok(())
    }

    pub async fn my_2fa_status(&self, user_id: &str) -> Result<TwoFactorStatus, Error> {
        let user = self
            .repo
            .get_user_by_id(user_id)
            .await?
            .ok_or_else(|| Error::not_found("User not found"))?;
        Ok(TwoFactorStatus {
            enabled: user.totp_enabled_at.is_some(),
            last_used_at: user.totp_last_used_at,
        })
    }

    pub async fn forgot_password(&self, email: String) -> Result<(), Error> {
        let expires_at = Utc::now() + chrono::Duration::hours(1);
        let token = generate_reset_token();

        // Best-effort: always return success to the caller to avoid
        // leaking whether the email exists. Only send mail when the row
        // was actually updated.
        let updated = self
            .repo
            .set_reset_token(&email, &token, expires_at)
            .await
            .unwrap_or(false);
        if updated {
            let reset_url = self
                .config
                .mail_reset_url_template
                .replace("{token}", &token);
            if let Err(e) = self.mailer.send_password_reset(&email, &reset_url).await {
                tracing::warn!(error = %e, email = %email, "password-reset mail dispatch failed");
            }
        }

        Ok(())
    }

    pub async fn reset_password(&self, token: String, new_password: String) -> Result<(), Error> {
        let user = self
            .repo
            .get_user_by_reset_token(&token)
            .await?
            .ok_or_else(|| Error::bad_request("invalid_token", "Invalid or expired reset token"))?;

        let id_str = user
            .id
            .as_ref()
            .map(|r| record_id_key_to_string(&r.key))
            .ok_or_else(|| Error::internal("auth", "User ID missing"))?;

        self.repo
            .reset_password_and_clear_token(&id_str, &new_password)
            .await
    }

    /// Issue a fresh email-verification token for the given user and email
    /// it to them via the configured mailer. No-op if the user is already
    /// verified.
    pub async fn request_email_verification(&self, user_id: &str) -> Result<(), Error> {
        let user = self
            .repo
            .get_user_by_id(user_id)
            .await?
            .ok_or_else(|| Error::not_found("User not found"))?;
        if user.is_verified {
            return Ok(());
        }

        let token = generate_reset_token();
        let expires_at = Utc::now() + chrono::Duration::hours(24);
        self.repo
            .set_email_verification_token(user_id, &token, expires_at)
            .await?;

        let verify_url = self
            .config
            .mail_verify_url_template
            .replace("{token}", &token);
        if let Err(e) = self
            .mailer
            .send_email_verification(&user.email, &verify_url)
            .await
        {
            tracing::warn!(error = %e, email = %user.email, "verification mail dispatch failed");
        }
        Ok(())
    }

    /// Consume a verification token, flipping `is_verified` if the token is
    /// valid and unexpired. Returns `true` on success, `false` for an
    /// unknown/expired token.
    pub async fn verify_email(&self, token: String) -> Result<bool, Error> {
        let updated = self.repo.consume_email_verification_token(&token).await?;
        Ok(updated.is_some())
    }

    pub async fn change_password(
        &self,
        user_id: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), Error> {
        let user = self
            .repo
            .get_user_by_id(user_id)
            .await?
            .ok_or_else(Error::wrong_credentials)?;

        if !verify_password(old_password, &user.password_hash) {
            return Err(Error::wrong_credentials());
        }

        self.repo.reset_password(user_id, new_password).await
    }

    pub async fn get_user_by_id(&self, id: &str) -> Result<UserResponse, Error> {
        let user = self
            .repo
            .get_user_by_id(id)
            .await?
            .ok_or_else(|| Error::not_found("User not found"))?;
        Ok(user.into())
    }

    pub async fn deactivate_user(&self, id: &str) -> Result<(), Error> {
        self.repo.deactivate_user(id).await
    }

    pub async fn get_user_stats(&self, user_id: &str) -> Result<UserStats, Error> {
        self.repo.get_user_stats(user_id).await
    }

    pub async fn get_reading_sessions(
        &self,
        user_id: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ReadingSessionResponse>, Error> {
        self.repo.get_reading_sessions(user_id, limit, offset).await
    }

    pub async fn get_following_authors(
        &self,
        user_id: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuthorResponse>, Error> {
        self.repo
            .get_following_authors(user_id, limit, offset)
            .await
    }

    pub async fn record_reading_session(
        &self,
        user_id: &str,
        dto: RecordReadingSessionDto,
    ) -> Result<ReadingSessionResponse, Error> {
        self.repo.record_reading_session(user_id, dto).await
    }
}
