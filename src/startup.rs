use actix_web::{web, App, HttpServer};
use actix_web::dev::Server;
use std::net::TcpListener;
use crate::routes::health_check::health_check_controller;
use crate::routes::subscriptions::subscribe_controller;

pub fn start_server(tcp_listener: TcpListener, db_connection_pool: sqlx::PgPool) -> Result<Server, std::io::Error> {
    println!("# Starting server");

    let db_connection_pool_data = web::Data::new(db_connection_pool);

    let http_server = HttpServer::new(move || {
        App::new()
            .route("/health-check", web::get().to(health_check_controller))
            .route("/subscriptions", web::post().to(subscribe_controller))
            .app_data(db_connection_pool_data.clone())
    })
    .listen(tcp_listener)?
    .run();

    Ok(http_server)
}
