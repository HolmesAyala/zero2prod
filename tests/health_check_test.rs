use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::{configuration, startup::start_server, telemetry};

pub struct TestApp {
    pub address: String,
    pub db_connection_pool: PgPool,
}

#[tokio::test]
async fn healt_check_success() {
    let test_app = spawn_server().await;
    let service_url = format!("{}/health_check", test_app.address);
    let client = reqwest::Client::new();

    let response = client
        .get(service_url.clone())
        .send()
        .await
        .expect(&format!("Unable to perform the request to {}", service_url));

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_success() {
    let test_app = spawn_server().await;

    let configuration = configuration::get_configuration().expect("Failed to read configuration");
    let pg_connect_options = configuration.database.with_db();
    let mut db_connection = PgConnection::connect_with(&pg_connect_options)
        .await
        .expect("Failed to connect to database");

    let service_url = format!("{}/subscriptions", test_app.address);
    let http_client = reqwest::Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let response = http_client
        .post(service_url.clone())
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect(&format!("Unable to perform the request to {}", service_url));

    assert_eq!(response.status().as_u16(), 200);

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&mut db_connection)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn given_missing_fields_then_subscribe_returns_400() {
    let test_app = spawn_server().await;
    let service_url = format!("{}/subscriptions", test_app.address);
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (requets_body, error_message) in test_cases {
        let response = client
            .post(service_url.clone())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(requets_body)
            .send()
            .await
            .expect(&format!("Unable to perform the request to {}", service_url));

        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        )
    }
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

async fn spawn_server() -> TestApp {
    Lazy::force(&TRACING);

    let tcp_listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind tcp listener");
    let address = format!("http://{}", tcp_listener.local_addr().unwrap().to_string());

    let mut configuration =
        configuration::get_configuration().expect("Failed to read configuration");
    configuration.database.database_name = Uuid::new_v4().to_string();

    let db_connection_pool = configure_database(&configuration.database).await;

    println!("# Address assigned: {}", address);

    let server =
        start_server(tcp_listener, db_connection_pool.clone()).expect("Unable to start the server");

    let _ = tokio::spawn(server);

    TestApp {
        address,
        db_connection_pool,
    }
}

async fn configure_database(database_settings: &configuration::DatabaseSettings) -> PgPool {
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
