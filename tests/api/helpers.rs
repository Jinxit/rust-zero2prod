use aws_config::TimeoutConfig;
use aws_sdk_sesv2 as ses;
use diesel::prelude::*;
use diesel::{Connection, PgConnection};
use once_cell::sync::Lazy;
use std::time::Duration;
use uuid::Uuid;
use zero2prod::configuration::{get_configuration, Settings};
use zero2prod::email::SesEmailClient;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".into();
    let subscriber_name = "test".into();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

pub async fn spawn_app() -> (String, Settings) {
    Lazy::force(&TRACING);

    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.application.port = Some(0);
    configuration.database.database_name = Uuid::new_v4().to_string();

    setup_database(&configuration);

    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address.");
    let timeout = Some(Duration::from_millis(200));
    let timeout_config = TimeoutConfig::new().with_api_call_timeout(timeout);
    let shared_config = aws_config::from_env()
        .timeout_config(timeout_config)
        .load()
        .await;
    let sns_client = ses::Client::new(&shared_config);
    let email_client = SesEmailClient::new(sns_client, sender_email);

    let (app, mut port) = zero2prod::startup::build(&configuration, Box::new(email_client))
        .await
        .unwrap();
    let _ = tokio::spawn(app.launch());
    (
        format!("http://127.0.0.1:{}", port.get().await),
        configuration,
    )
}

fn setup_database(configuration: &Settings) {
    let connection_string = configuration.database.connection_string_without_database();
    let connection =
        PgConnection::establish(&connection_string).expect("Failed to connect to Postgres.");

    diesel::sql_query(format!(
        "CREATE DATABASE \"{}\"",
        configuration.database.database_name
    ))
    .execute(&connection)
    .unwrap();

    let connection_string = configuration.database.connection_string();
    let connection =
        PgConnection::establish(&connection_string).expect("Failed to connect to Postgres.");

    diesel_migrations::run_pending_migrations(&connection).unwrap();
}
