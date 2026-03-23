use argon2::{Argon2, PasswordHash, PasswordVerifier};
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header, encode};

use crate::error::ServiceError;
use crate::models::{AuthRequest, AuthResponse, Claims};
use crate::repositories::user::UserRepository;

/// Service for handling user authentication and JWT token operations.
///
/// Provides methods for authenticating users, verifying JWT tokens, and
/// generating new tokens. Integrates with the user repository for credential
/// validation.
#[derive(Clone)]
pub struct AuthService {
    repository: UserRepository,
    jwt_secret: String,
}

impl AuthService {
    /// Creates a new `AuthService` instance.
    ///
    /// # Arguments
    /// * `repository` - User repository for credential verification
    /// * `jwt_secret` - Secret key for JWT signing and verification.
    pub fn new(repository: UserRepository, jwt_secret: String) -> Self {
        jsonwebtoken::crypto::rust_crypto::DEFAULT_PROVIDER
            .install_default()
            .expect("failed to install default crypto provider");

        if jwt_secret.as_bytes().len() < 32 {
            panic!(
                "JWT secret is too short: {} bytes (need >=32)",
                jwt_secret.as_bytes().len()
            );
        }

        Self {
            repository,
            jwt_secret,
        }
    }

    /// Authenticates a user and generates a JWT token.
    ///
    /// Verifies the user exists by email and generates a JWT token valid for 7 days.
    /// Validates the password
    ///
    /// # Arguments
    /// * `payload` - Authentication request with email and password
    ///
    /// # Returns
    /// An `AuthResponse` containing the JWT token on success
    pub async fn authenticate(&self, payload: AuthRequest) -> Result<AuthResponse, ServiceError> {
        // Find user by email
        let user = self
            .repository
            .find_by_email(&payload.email)
            .await?
            .ok_or(ServiceError::Unauthorized)?;

        // Validate password hash against payload.password
        let password_hash =
            PasswordHash::new(&user.password_hash).map_err(|_| ServiceError::Unauthorized)?;

        Argon2::default()
            .verify_password(payload.password.as_bytes(), &password_hash)
            .map_err(|_| ServiceError::Unauthorized)?;

        // Generate JWT token
        let token = self.generate_token(&user.user_id.to_string())?;

        Ok(AuthResponse { token })
    }

    /// Verifies and decodes a JWT token.
    ///
    /// Validates the token signature and expiration using the service's secret key.
    ///
    /// # Arguments
    /// * `token` - The JWT token string to verify
    ///
    /// # Returns
    /// The decoded `Claims` on success, or `InvalidToken` error if verification fails
    pub fn verify_token(&self, token: &str) -> Result<Claims, ServiceError> {
        jsonwebtoken::decode::<Claims>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &jsonwebtoken::Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|_| ServiceError::InvalidToken)
    }

    /// Generates a new JWT token for a user.
    ///
    /// Creates a signed JWT token with a 30-day expiration time.
    ///
    /// # Arguments
    /// * `user_id` - The user ID to encode in the token
    ///
    /// # Returns
    /// A JWT token string on success
    pub fn generate_token(&self, user_id: &str) -> Result<String, ServiceError> {
        let expiration = Utc::now()
            .checked_add_signed(chrono::Duration::days(30))
            .expect("valid timestamp")
            .timestamp() as usize;

        let claims = Claims {
            sub: user_id.to_owned(),
            exp: expiration,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| {
            // log the real error for debugging; in a production service you
            // might use the `log` crate instead of `eprintln!`.
            eprintln!("failed to generate JWT token: {:#?}", e);
            ServiceError::JWTGenFailed
        })
    }
}
