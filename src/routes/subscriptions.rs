use actix_web::{HttpResponse, web};
use chrono::Utc;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct SubscribeRequestBody {
    name: String,
    email: String,
}

pub async fn subscribe_controller(
    subscribe_request_body: web::Form<SubscribeRequestBody>, 
    db_connection_pool: web::Data<sqlx::PgPool>
) -> HttpResponse {
    let query_result = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        subscribe_request_body.email,
        subscribe_request_body.name,
        Utc::now()
    )
    .execute(db_connection_pool.get_ref())
    .await;

    match query_result {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            println!("Failed to execute query: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
