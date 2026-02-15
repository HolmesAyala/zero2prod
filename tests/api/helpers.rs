use crate::utils::get_first_link;
use once_cell::sync::Lazy;
use reqwest::Url;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::SocketAddr;
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::application::Application;
use zero2prod::configuration::get_configuration;
use zero2prod::{configuration, telemetry};

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub struct TestApp {
    pub socket_address: SocketAddr,
    pub address: String,
    pub db_connection_pool: PgPool,
    pub email_server: MockServer,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let email_request_body: serde_json::Value =
            serde_json::from_slice(&email_request.body).unwrap();

        let html_confirmation_link =
            get_first_link(email_request_body["HtmlBody"].as_str().unwrap());
        let mut html_confirmation_url = Url::parse(&html_confirmation_link).unwrap();
        html_confirmation_url
            .set_port(Some(self.socket_address.port()))
            .unwrap();

        assert_eq!("127.0.0.1", html_confirmation_url.host_str().unwrap());

        let plain_confirmation_link =
            get_first_link(email_request_body["HtmlBody"].as_str().unwrap());
        let mut plain_confirmation_url = Url::parse(&plain_confirmation_link).unwrap();
        plain_confirmation_url
            .set_port(Some(self.socket_address.port()))
            .unwrap();

        assert_eq!("127.0.0.1", plain_confirmation_url.host_str().unwrap());

        ConfirmationLinks {
            html: html_confirmation_url,
            plain_text: plain_confirmation_url,
        }
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

pub async fn spawn_server() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let configuration = {
        let mut test_configuration = get_configuration().expect("Failed to read configuration");
        test_configuration.database.database_name = Uuid::new_v4().to_string();
        test_configuration.application.port = 0;
        test_configuration.email_client.base_url = email_server.uri();
        test_configuration
    };

    configure_database(&configuration.database).await;

    let server = Application::build(configuration.clone())
        .await
        .expect("Failed to build server");
    let socket_addr = server.socket_addr().clone();
    let http_address = format!("http://{}", server.address());

    let _ = tokio::spawn(server.run_until_stopped());

    TestApp {
        socket_address: socket_addr,
        address: http_address,
        db_connection_pool: Application::get_connection_pool(&configuration.database),
        email_server,
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
