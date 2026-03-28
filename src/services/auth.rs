use argon2::{Argon2, PasswordHash, PasswordVerifier};
use chrono::Utc;
use jsonwebtoken::{Algorithm, EncodingKey, Header, Validation, encode};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::ServiceError;
use crate::models::{AuthRequest, AuthResponse, Claims, RefreshRequest};
use crate::repositories::{refresh_token::RefreshTokenRepository, user::UserRepository};

/// Maximum number of concurrent refresh tokens allowed per user.
/// Oldest tokens are pruned when this limit is exceeded on new login.
const MAX_REFRESH_TOKENS_PER_USER: usize = 10;

/// Lifetime of a refresh token in days.
const REFRESH_TOKEN_TTL_DAYS: i64 = 30;

/// Service for handling user authentication and JWT token operations.
///
/// Provides methods for authenticating users, verifying JWT tokens, and
/// generating new tokens. Integrates with the user repository for credential
/// validation.
///
/// # Initialization
///
/// The default crypto provider (`rustls`) **must** be installed exactly once
/// before constructing this service. Call
/// `AuthService::install_crypto_provider()` early in `main` (or your test
/// harness setup) before building the application state.
#[derive(Clone)]
pub struct AuthService {
    repository: UserRepository,
    refresh_repository: RefreshTokenRepository,
    jwt_secret: String,
}

impl AuthService {
    /// Installs the default `rustls` crypto provider.
    ///
    /// Must be called **once** at process startup (e.g. the top of `main`)
    /// before any `AuthService` instance is created. Calling it more than once
    /// will panic, so do not call it inside `AuthService::new`.
    ///
    /// ```rust
    /// // in main()
    /// AuthService::install_crypto_provider();
    /// ```
    pub fn install_crypto_provider() {
        jsonwebtoken::crypto::rust_crypto::DEFAULT_PROVIDER
            .install_default()
            .expect("failed to install default crypto provider");
    }

    /// Creates a new `AuthService` instance.
    ///
    /// # Panics
    /// Panics if `jwt_secret` is shorter than 32 bytes.
    ///
    /// # Arguments
    /// * `repository`         - User repository for credential verification
    /// * `refresh_repository` - Refresh-token repository
    /// * `jwt_secret`         - Secret key for JWT signing and verification
    pub fn new(
        repository: UserRepository,
        refresh_repository: RefreshTokenRepository,
        jwt_secret: String,
    ) -> Self {
        if jwt_secret.as_bytes().len() < 32 {
            panic!(
                "JWT secret is too short: {} bytes (need >=32)",
                jwt_secret.as_bytes().len()
            );
        }

        Self {
            repository,
            refresh_repository,
            jwt_secret,
        }
    }

    /// Authenticates a user and returns a short-lived access token plus a
    /// long-lived refresh token.
    ///
    /// Verifies the user exists by email, validates the password, and issues
    /// both tokens. Older refresh tokens are pruned if the per-user cap is
    /// reached.
    ///
    /// # Arguments
    /// * `payload` - Authentication request containing email and password
    ///
    /// # Returns
    /// An `AuthResponse` containing the access and refresh tokens on success.
    pub async fn authenticate(&self, payload: AuthRequest) -> Result<AuthResponse, ServiceError> {
        // Find user by email
        let user = self
            .repository
            .find_by_email(&payload.email)
            .await?
            .ok_or(ServiceError::Unauthorized)?;

        // Validate password
        let password_hash =
            PasswordHash::new(&user.password_hash).map_err(|_| ServiceError::Unauthorized)?;

        Argon2::default()
            .verify_password(payload.password.as_bytes(), &password_hash)
            .map_err(|_| ServiceError::Unauthorized)?;

        let access_token = self.generate_access_token(user.user_id)?;
        let refresh_token = self.issue_refresh_token(user.user_id).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
        })
    }

    /// Validates a refresh token, rotates it, and returns a new token pair.
    ///
    /// The consumed token is deleted immediately (rotation). If the token is
    /// expired it is also cleaned up before returning an error.
    pub async fn refresh(&self, payload: RefreshRequest) -> Result<AuthResponse, ServiceError> {
        let hash = Self::hash_token(&payload.refresh_token);
        let record = self
            .refresh_repository
            .find_by_hash(&hash)
            .await?
            .ok_or(ServiceError::InvalidToken)?;

        // Rotate: delete the consumed token before issuing a new one
        self.refresh_repository.delete(record.token_id).await?;

        let access_token = self.generate_access_token(record.user_id)?;
        let refresh_token = self.issue_refresh_token(record.user_id).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
        })
    }

    /// Logs the user out.
    ///
    /// * If `token` is `Some`, only that device's refresh token is revoked.
    /// * If `token` is `None`, **all** refresh tokens for the user are revoked
    ///   (logout from every device).
    ///
    /// # Note on access tokens
    /// Outstanding access tokens remain valid until they expire (up to 15
    /// minutes). For stricter revocation, introduce a short-lived token
    /// blocklist backed by Redis or a similar fast store.
    pub async fn logout(&self, user_id: Uuid, token: Option<&str>) -> Result<(), ServiceError> {
        match token {
            Some(raw) => {
                let hash = Self::hash_token(raw);
                self.refresh_repository.delete_by_hash(&hash).await?;
            }
            None => {
                self.refresh_repository.delete_all_for_user(user_id).await?;
            }
        }
        Ok(())
    }

    /// Issues a new opaque refresh token, stores its SHA-256 hash in the DB,
    /// and returns the raw (unhashed) token to the client.
    ///
    /// Enforces `MAX_REFRESH_TOKENS_PER_USER`: if the user already holds that
    /// many active tokens, the oldest ones are pruned before inserting the new
    /// one.
    async fn issue_refresh_token(&self, user_id: Uuid) -> Result<String, ServiceError> {
        // Prune oldest tokens if the user is at the cap
        self.refresh_repository
            .prune_oldest_for_user(user_id, MAX_REFRESH_TOKENS_PER_USER)
            .await?;

        let raw = Uuid::new_v4().to_string();
        let hash = Self::hash_token(&raw);

        // FIX: Refresh tokens are now long-lived (30 days), not 15 minutes
        let expires_at = Utc::now()
            .checked_add_signed(chrono::Duration::days(REFRESH_TOKEN_TTL_DAYS))
            .expect("valid timestamp");

        self.refresh_repository
            .create(user_id, &hash, expires_at)
            .await?;

        Ok(raw)
    }

    fn hash_token(raw: &str) -> String {
        let digest = Sha256::digest(raw.as_bytes());
        hex::encode(digest)
    }

    /// Verifies and decodes a JWT access token.
    ///
    /// Validates the token signature, expiration, and algorithm (HS256).
    ///
    /// # Arguments
    /// * `token` - The JWT token string to verify
    ///
    /// # Returns
    /// The decoded `Claims` on success, or `InvalidToken` if verification fails.
    pub fn verify_token(&self, token: &str) -> Result<Claims, ServiceError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        jsonwebtoken::decode::<Claims>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
        .map(|data| data.claims)
        .map_err(|_| ServiceError::InvalidToken)
    }

    /// Generates a short-lived access JWT (15 minutes) for the given user.
    ///
    /// # Arguments
    /// * `user_id` - The UUID of the user to encode in the token
    ///
    /// # Returns
    /// A signed JWT string on success.
    pub fn generate_access_token(&self, user_id: Uuid) -> Result<String, ServiceError> {
        let expiration = Utc::now()
            .checked_add_signed(chrono::Duration::minutes(15))
            .expect("valid timestamp")
            .timestamp() as usize;

        let claims = Claims {
            sub: user_id.to_string(),
            exp: expiration,
        };

        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| {
            eprintln!("failed to generate access token: {:#?}", e);
            ServiceError::JWTGenFailed
        })
    }
}