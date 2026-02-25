use crate::authentication::{AuthError, Credentials, validate_credentials};
use crate::session_state::TypedSession;
use crate::utils::error_chain_fmt;
use actix_web::error::InternalError;
use actix_web::http::StatusCode;
use actix_web::http::header::LOCATION;
use actix_web::{HttpResponse, ResponseError, web};
use actix_web_flash_messages::FlashMessage;
use secrecy::SecretString;
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: SecretString,
}

#[tracing::instrument(
    name = "login",
    skip(request_body, db_pool, session),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    request_body: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: request_body.0.username,
        password: request_body.0.password,
    };

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    let result = validate_credentials(credentials, &db_pool).await;

    match result {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

            session.renew();
            session
                .insert_user_id(user_id)
                .map_err(|error| login_redirect(LoginError::UnexpectedError(error.into())))?;

            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/admin/dashboard"))
                .finish())
        }
        Err(error) => {
            let error_mapped = match error {
                AuthError::InvalidCredentials(error) => LoginError::AuthError(error),
                AuthError::UnexpectedError(error) => LoginError::UnexpectedError(error),
            };

            Err(login_redirect(error_mapped))
        }
    }
}

fn login_redirect(error: LoginError) -> InternalError<LoginError> {
    FlashMessage::error(error.to_string()).send();

    let response = HttpResponse::SeeOther()
        .insert_header((LOCATION, "/login"))
        .finish();

    InternalError::from_response(error, response)
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {
    fn status_code(&self) -> StatusCode {
        match self {
            LoginError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            LoginError::AuthError(_) => StatusCode::UNAUTHORIZED,
        }
    }
}
