use actix_web::{HttpResponse, web};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "confirm_subscription", skip(parameters, db_connection_pool))]
pub async fn confirm_subscription(
    parameters: web::Query<Parameters>,
    db_connection_pool: web::Data<PgPool>,
) -> HttpResponse {
    let subscriber_id =
        match get_subscriber_id_from_token(&db_connection_pool, &parameters.subscription_token)
            .await
        {
            Ok(subscriber_id) => subscriber_id,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };

    match subscriber_id {
        None => HttpResponse::Unauthorized().finish(),
        Some(subscriber_id) => {
            if confirm_subscriber(&db_connection_pool, subscriber_id)
                .await
                .is_err()
            {
                return HttpResponse::InternalServerError().finish();
            }

            HttpResponse::Ok().finish()
        }
    }
}

#[tracing::instrument(
    name = "get_subscriber_id_from_token",
    skip(db_connection_pool, subscription_token)
)]
async fn get_subscriber_id_from_token(
    db_connection_pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let record = sqlx::query!(
        "SELECT subscriber_id FROM subscriptions_tokens WHERE subscription_token = $1",
        subscription_token
    )
    .fetch_optional(db_connection_pool)
    .await
    .map_err(|error| {
        tracing::error!("Failed to fetch subscriber_id from token: {}", error);
        error
    })?;

    Ok(record.map(|record| record.subscriber_id))
}

#[tracing::instrument(name = "confirm_subscriber", skip(db_connection_pool, subscriber_id))]
async fn confirm_subscriber(
    db_connection_pool: &PgPool,
    subscriber_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE subscriptions SET status = 'confirmed' WHERE id = $1",
        subscriber_id
    )
    .execute(db_connection_pool)
    .await
    .map_err(|error| {
        tracing::error!("Failed to update subscription status: {}", error);
        error
    })?;

    Ok(())
}
