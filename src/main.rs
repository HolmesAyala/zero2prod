use std::net::TcpListener;
use sqlx::{PgPool};
use zero2prod::{configuration, startup::start_server};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let configuration = configuration::get_configuration()
        .expect("Failed to read configuration");
    let db_connection_pool = PgPool::connect(
        &configuration.database.connection_string()
    ).await
        .expect("Failed to connect to the database");

    let tcp_listener = TcpListener::bind(format!("127.0.0.1:{}", configuration.application_port))
        .expect("Failed to bind tcp listener");

    start_server(tcp_listener, db_connection_pool)?.await
}
