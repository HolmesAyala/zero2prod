use crate::routes::health_check::health_check_controller;
use crate::routes::subscriptions::subscribe_controller;
use actix_web::dev::Server;
use actix_web::{App, HttpServer, web};
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub fn start_server(
    tcp_listener: TcpListener,
    db_connection_pool: sqlx::PgPool,
) -> Result<Server, std::io::Error> {
    let db_connection_pool_data = web::Data::new(db_connection_pool);

    let http_server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health-check", web::get().to(health_check_controller))
            .route("/subscriptions", web::post().to(subscribe_controller))
            .app_data(db_connection_pool_data.clone())
    })
    .listen(tcp_listener)?
    .run();

    Ok(http_server)
}
