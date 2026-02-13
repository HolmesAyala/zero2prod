use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::{configuration, telemetry};
use zero2prod::configuration::get_configuration;
use zero2prod::application::{Application};

pub struct TestApp {
    pub address: String,
    pub db_connection_pool: PgPool,
}

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = telemetry::get_tracing_subscriber(
            subscriber_name,
            default_filter_level,
            std::io::stdout,
        );
        telemetry::init_tracing_subscriber(subscriber);
    } else {
        let subscriber =
            telemetry::get_tracing_subscriber(subscriber_name, default_filter_level, std::io::sink);
        telemetry::init_tracing_subscriber(subscriber);
    }
});

pub async fn spawn_server() -> TestApp {
    Lazy::force(&TRACING);

    let configuration = {
        let mut test_configuration = get_configuration().expect("Failed to read configuration");
        test_configuration.database.database_name = Uuid::new_v4().to_string();
        test_configuration.application.port = 0;
        test_configuration
    };

    configure_database(&configuration.database).await;

    let server = Application::build(configuration.clone()).await.expect("Failed to build server");
    let http_address = format!("http://{}", server.address());

    let _ = tokio::spawn(server.run_until_stopped());

    TestApp {
        address: http_address,
        db_connection_pool: Application::get_connection_pool(&configuration.database),
    }
}

pub async fn configure_database(database_settings: &configuration::DatabaseSettings) -> PgPool {
    let mut db_connection = PgConnection::connect_with(&database_settings.without_db())
        .await
        .expect("Failed to connect to database");

    db_connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, database_settings.database_name).as_str())
        .await
        .expect("Failed to create database");

    let db_connection_pool = PgPool::connect_with(database_settings.with_db())
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("./migrations")
        .run(&db_connection_pool)
        .await
        .expect("Failed to migrate the database");

    db_connection_pool
}
