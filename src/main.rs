use sqlx::PgPool;
use std::net::TcpListener;
use zero2prod::{
    configuration,
    startup::start_server,
    telemetry::{get_tracing_subscriber, init_tracing_subscriber},
};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let tracing_subscriber =
        get_tracing_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_tracing_subscriber(tracing_subscriber);

    let configuration = configuration::get_configuration().expect("Failed to read configuration");
    let db_connection_pool = PgPool::connect_lazy_with(configuration.database.with_db());

    let tcp_listener = TcpListener::bind(configuration.application.address())
        .expect("Failed to bind tcp listener");

    start_server(tcp_listener, db_connection_pool)?.await
}
