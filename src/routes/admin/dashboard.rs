use crate::session_state::TypedSession;
use actix_web::error::ErrorInternalServerError;
use actix_web::http::header::{ContentType, LOCATION};
use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn admin_dashboard(
    session: TypedSession,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Ok(Some(user_id)) = session
        .get_user_id()
        .map_err(|error| ErrorInternalServerError(error))
    {
        get_username(user_id, &db_pool)
            .await
            .map_err(|error| ErrorInternalServerError(error))?
    } else {
        return Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION, "/login"))
            .finish());
    };

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
                <html lang="en">
                <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8">
                <title>Admin dashboard</title>
                </head>
                <body>
                <p>Welcome {username}!</p>
                <p>Available actions:</p>
                    <ol>
                        <li><a href="/admin/password">Change password</a></li>
                    </ol>
                </body>
                </html>
            "#
        )))
}

async fn get_username(user_id: Uuid, db_pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(r#"SELECT username FROM users WHERE user_id = $1"#, user_id)
        .fetch_one(db_pool)
        .await
        .context("failed to execute query to retrieve username")?;

    Ok(row.username)
}
