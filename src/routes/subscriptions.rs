use actix_web::{HttpResponse, web};
use chrono::Utc;
use uuid::Uuid;

#[derive(Debug, serde::Deserialize)]
pub struct SubscribeRequestBody {
    name: String,
    email: String,
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
    match insert_subscriber(&db_connection_pool, &subscribe_request_body).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(
    name = "insert_subscriber",
    skip(db_connection_pool, subscribe_request_body)
)]
pub async fn insert_subscriber(
    db_connection_pool: &sqlx::PgPool,
    subscribe_request_body: &SubscribeRequestBody,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        subscribe_request_body.email,
        subscribe_request_body.name,
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
