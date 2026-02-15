use crate::application::ApplicationBaseUrl;
use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use crate::email_client::EmailClient;
use actix_web::{HttpResponse, web};
use chrono::Utc;
use rand::distr::{Alphanumeric, SampleString};
use sqlx::{Executor, Postgres, Transaction};
use uuid::Uuid;

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
) -> HttpResponse {
    let new_subscriber: NewSubscriber = match subscribe_request_body.0.try_into() {
        Ok(new_subscriber) => new_subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let mut transaction = match db_connection_pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let subscriber_id = match insert_subscriber(&mut transaction, &new_subscriber).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let subscription_token = generate_subscription_token();

    if store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    if transaction.commit().await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    if send_confirmation_email(
        &email_client,
        new_subscriber,
        &application_base_url,
        &subscription_token,
    )
    .await
    .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
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
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"INSERT INTO subscriptions_tokens (subscription_token, subscriber_id) VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id,
    );

    transaction.execute(query).await.map_err(|error| {
        tracing::error!("Failed to execute query: {:?}", error);
        error
    })?;

    Ok(())
}

fn generate_subscription_token() -> String {
    let mut rng = rand::rng();

    Alphanumeric.sample_string(&mut rng, 25)
}
