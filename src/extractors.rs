use crate::{error::ServiceError, services::auth_service::AuthService};
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use std::future::Future;
use uuid::Uuid;

pub struct AuthenticatedUser {
    pub user_id: Uuid,
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
    AuthService: FromRef<S>,
{
    type Rejection = ServiceError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let TypedHeader(Authorization(bearer)) =
                TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
                    .await
                    .map_err(|_| ServiceError::Unauthorized)?;

            let auth_service = AuthService::from_ref(state);
            let claims = auth_service.verify_token(bearer.token())?;

            Ok(AuthenticatedUser {
                user_id: Uuid::parse_str(&claims.sub).map_err(|_| ServiceError::Unauthorized)?,
            })
        }
    }
}
