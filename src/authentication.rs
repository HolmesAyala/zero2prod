use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: SecretString,
}

#[tracing::instrument(name = "validate_credentials", skip(credentials, db_connection_pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    db_connection_pool: &PgPool,
) -> Result<uuid::Uuid, AuthError> {
    let mut user_id = None;
    let mut password_hash = SecretString::from(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    if let Some((user_id_stored, password_hash_stored)) =
        get_stored_credentials(credentials.username.as_str(), db_connection_pool).await?
    {
        user_id = Some(user_id_stored);
        password_hash = password_hash_stored;
    }

    let current_span = tracing::Span::current();

    tokio::task::spawn_blocking(move || {
        current_span.in_scope(|| verify_password_hash(password_hash, credentials.password))
    })
    .await
    .context("Failed to spawn blocking task")??;

    user_id
        .ok_or_else(|| anyhow::anyhow!("Unknown username"))
        .map_err(AuthError::InvalidCredentials)
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
) -> Result<(), AuthError> {
    let stored_password_hash_as_phc = PasswordHash::new(&password_hash_stored.expose_secret())
        .context("Failed to parse hash in PHC string format.")
        .map_err(AuthError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            password_to_verify.expose_secret().as_bytes(),
            &stored_password_hash_as_phc,
        )
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)
}
