use actix_web::{HttpResponse, web};
use chrono::Utc;
use uuid::Uuid;
use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};

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
    skip(subscribe_request_body, db_connection_pool),
    fields(
        subscribe_request_body = ?&subscribe_request_body
    )
)]
pub async fn subscribe_controller(
    subscribe_request_body: web::Form<SubscribeRequestBody>,
    db_connection_pool: web::Data<sqlx::PgPool>,
) -> HttpResponse {
    let new_subscriber: NewSubscriber = match subscribe_request_body.0.try_into() {
        Ok(new_subscriber) => new_subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    match insert_subscriber(&db_connection_pool, &new_subscriber).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(
    name = "insert_subscriber",
    skip(db_connection_pool, new_subscriber)
)]
pub async fn insert_subscriber(
    db_connection_pool: &sqlx::PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(db_connection_pool)
    .await
    .map_err(|error| {
        tracing::error!("Failed to execute query: {:?}", error);
        error
    })?;

    Ok(())
}
