use zero2prod::application::Application;
use zero2prod::{
    configuration,
    telemetry::{get_tracing_subscriber, init_tracing_subscriber},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tracing_subscriber =
        get_tracing_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_tracing_subscriber(tracing_subscriber);

    let configuration = configuration::get_configuration().expect("Failed to read configuration");

    let http_server = Application::build(configuration).await?;

    http_server.run_until_stopped().await?;

    Ok(())
}
