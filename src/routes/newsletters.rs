use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::utils::error_chain_fmt;
use actix_web::http::header::{HeaderMap, HeaderValue};
use actix_web::http::{header, StatusCode};
use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use anyhow::{Context};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::Engine;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::PgPool;
use std::fmt::Debug;

#[derive(Debug, Deserialize)]
pub struct PublishNewsletterRequestBody {
    title: String,
    content: PublishNewsletterRequestBodyContent,
}

#[derive(Debug, Deserialize)]
pub struct PublishNewsletterRequestBodyContent {
    html: String,
    text: String,
}

#[derive(thiserror::Error)]
pub enum PublishNewsletterError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for PublishNewsletterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishNewsletterError {
    fn status_code(&self) -> StatusCode {
        match self {
            PublishNewsletterError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            PublishNewsletterError::AuthError(_) => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            PublishNewsletterError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
            PublishNewsletterError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);

                let header_value =
                    HeaderValue::from_str(r#"Basic realm="publish_newsletter""#).unwrap();

                let response_headers: &mut HeaderMap = response.headers_mut();

                response_headers.insert(header::WWW_AUTHENTICATE, header_value);

                response
            }
        }
    }
}

#[tracing::instrument(
    name = "publish_newsletter",
    skip(db_connection_pool, email_client, http_request)
)]
pub async fn publish_newsletter(
    request_body: web::Json<PublishNewsletterRequestBody>,
    db_connection_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    http_request: HttpRequest,
) -> Result<HttpResponse, PublishNewsletterError> {
    let basic_credentials =
        get_basic_credentials(http_request.headers()).map_err(PublishNewsletterError::AuthError)?;

    validate_credentials(basic_credentials, db_connection_pool.as_ref()).await?;

    let subscribers = get_confirmed_subscribers(&db_connection_pool).await?;

    for subscriber_result in subscribers {
        match subscriber_result {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &request_body.title,
                        &request_body.content.html,
                        &request_body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber. Their stored contact details are invalid",
                )
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}

struct Credentials {
    username: String,
    password: SecretString,
}

#[tracing::instrument(name = "get_basic_credentials", skip(headers))]
fn get_basic_credentials(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The Authorization header is missing")?
        .to_str()
        .context("The Authorization header is not a valid UTF-8 string")?;

    let base64encoded_content = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'")?;

    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_content)
        .context("Failed to decode the basic credentials from base64")?;

    let decoded_credentials = String::from_utf8(decoded_bytes.to_vec())
        .context("The Basic credentials is not valid UTF-8")?;

    let mut credentials = decoded_credentials.splitn(2, ':');

    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth"))?
        .to_owned();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth"))?
        .to_owned();

    Ok(Credentials {
        username,
        password: SecretString::from(password),
    })
}

#[tracing::instrument(name = "validate_credentials", skip(credentials, db_connection_pool))]
async fn validate_credentials(
    credentials: Credentials,
    db_connection_pool: &PgPool,
) -> Result<uuid::Uuid, PublishNewsletterError> {
    let mut user_id = None;
    let mut password_hash = SecretString::from(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno".to_string()
    );

    if let Some((user_id_stored, password_hash_stored)) =
        get_stored_credentials(credentials.username.as_str(), db_connection_pool)
            .await
            .map_err(PublishNewsletterError::UnexpectedError)? 
    {
        user_id = Some(user_id_stored);
        password_hash = password_hash_stored;
    }

    let current_span = tracing::Span::current();

    tokio::task::spawn_blocking(move || {
        current_span.in_scope(|| verify_password_hash(password_hash, credentials.password))
    })
    .await
    .context("Failed to spawn blocking task")
    .map_err(PublishNewsletterError::UnexpectedError)??;

    user_id.ok_or_else(|| 
        PublishNewsletterError::UnexpectedError(anyhow::anyhow!("Unknown username"))
    )
}

#[tracing::instrument(name = "get_stored_credentials", skip(db_connection_pool))]
async fn get_stored_credentials(
    username: &str,
    db_connection_pool: &PgPool,
) -> Result<Option<(uuid::Uuid, SecretString)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
            SELECT user_id, password_hash
            FROM users
            WHERE username = $1
        "#,
        username
    )
    .fetch_optional(db_connection_pool)
    .await
    .context("Failed to perform the query to retrieve stored credentials.")?;

    let fields = row.map(|row| (row.user_id, SecretString::from(row.password_hash)));

    Ok(fields)
}

#[tracing::instrument(
    name = "verify_password_hash",
    skip(password_hash_stored, password_to_verify)
)]
fn verify_password_hash(
    password_hash_stored: SecretString,
    password_to_verify: SecretString,
) -> Result<(), PublishNewsletterError> {
    let stored_password_hash_as_phc = PasswordHash::new(&password_hash_stored.expose_secret())
        .context("Failed to parse hash in PHC string format.")
        .map_err(PublishNewsletterError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            password_to_verify.expose_secret().as_bytes(),
            &stored_password_hash_as_phc,
        )
        .context("Invalid password")
        .map_err(PublishNewsletterError::AuthError)
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "get_confirmed_subscribers", skip(db_connection_pool))]
async fn get_confirmed_subscribers(
    db_connection_pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let rows = sqlx::query!("SELECT email FROM subscriptions WHERE status = 'confirmed'")
        .fetch_all(db_connection_pool)
        .await?;

    let rows_mapped = rows
        .into_iter()
        .map(|row| match SubscriberEmail::parse(row.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error)),
        })
        .collect();

    Ok(rows_mapped)
}
