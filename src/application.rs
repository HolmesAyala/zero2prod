use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::confirm_subscription::confirm_subscription;
use crate::routes::health_check::health_check_controller;
use crate::routes::subscriptions::subscribe_controller;
use actix_web::dev::Server;
use actix_web::{App, HttpServer, web};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
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
        db_connection_pool: sqlx::PgPool,
        email_client: EmailClient,
        base_url: String,
    ) -> Result<Server, std::io::Error> {
        let db_connection_pool_data = web::Data::new(db_connection_pool);
        let email_client_data = web::Data::new(email_client);
        let application_base_url = web::Data::new(ApplicationBaseUrl(base_url));

        let http_server = HttpServer::new(move || {
            App::new()
                .wrap(TracingLogger::default())
                .route("/health_check", web::get().to(health_check_controller))
                .route("/subscriptions", web::post().to(subscribe_controller))
                .route(
                    "/subscriptions/confirm",
                    web::get().to(confirm_subscription),
                )
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
