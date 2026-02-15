use crate::application::ApplicationBaseUrl;
use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use crate::email_client::EmailClient;
use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use chrono::Utc;
use rand::distr::{Alphanumeric, SampleString};
use sqlx::{Executor, Postgres, Transaction};
use std::error::Error;
use std::fmt::{Debug, Display};
use uuid::Uuid;

pub struct StoreTokenError(sqlx::Error);

impl Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while trying to store a subscription token."
        )
    }
}

impl Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

fn error_chain_fmt(error: &impl Error, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "{}\n", error)?;
    let mut current = error.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

pub enum SubscribeError {
    ValidationError(String),
    StoreTokenError(StoreTokenError),
    SendEmailError(reqwest::Error),
    PoolError(sqlx::Error),
    InsertSubscriberError(sqlx::Error),
    TransactionCommitError(sqlx::Error),
}

impl Display for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscribeError::ValidationError(e) => write!(f, "{}", e),
            SubscribeError::StoreTokenError(_) => write!(
                f,
                "Failed to store the confirmation token for the new subscriber"
            ),
            SubscribeError::SendEmailError(_) => write!(f, "Failed to send the confirmation email"),
            SubscribeError::PoolError(_) => {
                write!(f, "Failed to get a database connection from the pool")
            }
            SubscribeError::InsertSubscriberError(_) => {
                write!(f, "Failed to insert subscriber in the database")
            }
            SubscribeError::TransactionCommitError(_) => write!(
                f,
                "Failed to commit the transaction to store a new subscriber"
            ),
        }
    }
}

impl Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl Error for SubscribeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SubscribeError::ValidationError(_) => None,
            SubscribeError::StoreTokenError(error) => Some(error),
            SubscribeError::SendEmailError(error) => Some(error),
            SubscribeError::PoolError(error) => Some(error),
            SubscribeError::InsertSubscriberError(error) => Some(error),
            SubscribeError::TransactionCommitError(error) => Some(error),
        }
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::PoolError(_)
            | SubscribeError::TransactionCommitError(_)
            | SubscribeError::InsertSubscriberError(_)
            | SubscribeError::StoreTokenError(_)
            | SubscribeError::SendEmailError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<String> for SubscribeError {
    fn from(error: String) -> Self {
        SubscribeError::ValidationError(error)
    }
}

impl From<StoreTokenError> for SubscribeError {
    fn from(err: StoreTokenError) -> Self {
        SubscribeError::StoreTokenError(err)
    }
}

impl From<reqwest::Error> for SubscribeError {
    fn from(err: reqwest::Error) -> Self {
        SubscribeError::SendEmailError(err)
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct SubscribeRequestBody {
    name: String,
    email: String,
}

impl TryFrom<SubscribeRequestBody> for NewSubscriber {
    type Error = String;

    fn try_from(value: SubscribeRequestBody) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(NewSubscriber { email, name })
    }
}

#[tracing::instrument(
    name = "subscribe_controller",
    skip(subscribe_request_body, db_connection_pool, email_client, application_base_url),
    fields(
        subscribe_request_body = ?&subscribe_request_body
    )
)]
pub async fn subscribe_controller(
    subscribe_request_body: web::Form<SubscribeRequestBody>,
    db_connection_pool: web::Data<sqlx::PgPool>,
    email_client: web::Data<EmailClient>,
    application_base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let new_subscriber: NewSubscriber = subscribe_request_body.0.try_into()?;

    let mut transaction = db_connection_pool
        .begin()
        .await
        .map_err(SubscribeError::PoolError)?;

    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .map_err(SubscribeError::InsertSubscriberError)?;
    let subscription_token = generate_subscription_token();

    store_token(&mut transaction, subscriber_id, &subscription_token).await?;

    transaction
        .commit()
        .await
        .map_err(SubscribeError::TransactionCommitError)?;

    send_confirmation_email(
        &email_client,
        new_subscriber,
        &application_base_url,
        &subscription_token,
    )
    .await?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "insert_subscriber", skip(transaction, new_subscriber))]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();

    let query = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    );

    transaction.execute(query).await.map_err(|error| {
        tracing::error!("Failed to execute query: {:?}", error);
        error
    })?;

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "send_confirmation_email",
    skip(email_client, new_subscriber, application_base_url)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    application_base_url: &ApplicationBaseUrl,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        application_base_url.0, subscription_token
    );

    let html_content = format!(
        "Welcome to our newsletter!<br />\
        Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );

    let text_content = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );

    email_client
        .send_email(
            new_subscriber.email,
            "Welcome!",
            &html_content,
            &text_content,
        )
        .await
}

#[tracing::instrument(
    name = "store_token",
    skip(transaction, subscriber_id, subscription_token)
)]
async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    let query = sqlx::query!(
        r#"INSERT INTO subscriptions_tokens (subscription_token, subscriber_id) VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id,
    );

    transaction.execute(query).await.map_err(|error| {
        tracing::error!("Failed to execute query: {:?}", error);
        StoreTokenError(error)
    })?;

    Ok(())
}

fn generate_subscription_token() -> String {
    let mut rng = rand::rng();

    Alphanumeric.sample_string(&mut rng, 25)
}
