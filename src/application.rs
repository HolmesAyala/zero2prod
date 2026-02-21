use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::confirm_subscription::confirm_subscription;
use crate::routes::health_check::health_check_controller;
use crate::routes::home::home;
use crate::routes::login::get::login_form;
use crate::routes::login::post::login;
use crate::routes::newsletters::publish_newsletter;
use crate::routes::subscriptions::subscribe_controller;
use actix_web::cookie::Key;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use actix_web_flash_messages::storage::CookieMessageStore;
use actix_web_flash_messages::FlashMessagesFramework;
use secrecy::{ExposeSecret, SecretString};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::{SocketAddr, TcpListener};
use tracing_actix_web::TracingLogger;

pub struct ApplicationBaseUrl(pub String);

pub struct Application {
    socket_addr: SocketAddr,
    address: String,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        let connection_pool = Application::get_connection_pool(&configuration.database);

        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address");

        let timeout = configuration.email_client.timeout();

        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
        );

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );

        let tcp_listener = TcpListener::bind(&address)?;
        let address_assigned = tcp_listener.local_addr()?;

        let server = Application::start_server(
            tcp_listener,
            connection_pool,
            email_client,
            configuration.application.base_url,
            configuration.application.hmac_secret,
        )?;

        Ok(Self {
            socket_addr: address_assigned,
            address: address_assigned.to_string(),
            server,
        })
    }

    pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
        PgPoolOptions::new().connect_lazy_with(configuration.with_db())
    }

    fn start_server(
        tcp_listener: TcpListener,
        db_connection_pool: PgPool,
        email_client: EmailClient,
        base_url: String,
        hmac_secret: SecretString,
    ) -> Result<Server, std::io::Error> {
        let db_connection_pool_data = web::Data::new(db_connection_pool);
        let email_client_data = web::Data::new(email_client);
        let application_base_url = web::Data::new(ApplicationBaseUrl(base_url));

        let message_store =
            CookieMessageStore::builder(Key::from(hmac_secret.expose_secret().as_bytes())).build();
        let message_framework = FlashMessagesFramework::builder(message_store).build();

        let http_server = HttpServer::new(move || {
            App::new()
                .wrap(message_framework.clone())
                .wrap(TracingLogger::default())
                .route("/", web::get().to(home))
                .route("/health_check", web::get().to(health_check_controller))
                .route("/login", web::get().to(login_form))
                .route("/login", web::post().to(login))
                .route("/subscriptions", web::post().to(subscribe_controller))
                .route(
                    "/subscriptions/confirm",
                    web::get().to(confirm_subscription),
                )
                .route("/newsletters", web::post().to(publish_newsletter))
                .app_data(db_connection_pool_data.clone())
                .app_data(email_client_data.clone())
                .app_data(application_base_url.clone())
        })
        .listen(tcp_listener)?
        .run();

        Ok(http_server)
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn socket_addr(&self) -> &SocketAddr {
        &self.socket_addr
    }
}
