use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::utils::error_chain_fmt;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError, web};
use anyhow::Context;
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
        }
    }
}

#[tracing::instrument(name = "publish_newsletter", skip(db_connection_pool, email_client))]
pub async fn publish_newsletter(
    request_body: web::Json<PublishNewsletterRequestBody>,
    db_connection_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, PublishNewsletterError> {
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
