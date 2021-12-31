use aws_config::TimeoutConfig;
use aws_sdk_sesv2 as ses;
use std::time::Duration;
use zero2prod::configuration::get_configuration;
use zero2prod::email::SesEmailClient;
use zero2prod::startup::build;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address.");

    let timeout = Some(Duration::from_millis(
        configuration.email_client.timeout_milliseconds,
    ));
    let timeout_config = TimeoutConfig::new().with_api_call_timeout(timeout);
    let shared_config = aws_config::from_env()
        .timeout_config(timeout_config)
        .load()
        .await;
    let sns_client = ses::Client::new(&shared_config);
    let email_client = SesEmailClient::new(sns_client, sender_email);

    build(&configuration, Box::new(email_client))
        .await?
        .0
        .launch()
        .await
}
