use aws_config::TimeoutConfig;
use aws_sdk_sesv2 as ses;
use diesel::prelude::*;
use diesel::{Connection, PgConnection};
use once_cell::sync::Lazy;
use std::time::Duration;
use uuid::Uuid;
use zero2prod::configuration::{get_configuration, Settings};
use zero2prod::email::SesEmailClient;
use zero2prod::models::*;
use zero2prod::schema::subscriptions::dsl::*;
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

#[tokio::test]
async fn health_check_works() {
    // arrange
    let (address, _) = spawn_app().await;

    let client = reqwest::Client::new();

    // act
    let response = client
        .get(format!("{}/health_check", address))
        .send()
        .await
        .expect("Failed to execute request.");

    // assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // arrange
    let (address, configuration) = spawn_app().await;

    let connection_string = configuration.database.connection_string();
    let connection =
        PgConnection::establish(&connection_string).expect("Failed to connect to Postgres.");

    let client = reqwest::Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // act
    let response = client
        .post(&format!("{}/subscriptions", &address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // assert
    assert_eq!(200, response.status().as_u16());

    let saved = subscriptions
        .first::<Subscription>(&connection)
        .expect("Result set was empty.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn subscribe_returns_a_500_for_duplicate_email() {
    // arrange
    let (address, configuration) = spawn_app().await;

    let connection_string = configuration.database.connection_string();
    let connection =
        PgConnection::establish(&connection_string).expect("Failed to connect to Postgres.");

    let client = reqwest::Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // act
    client
        .post(&format!("{}/subscriptions", &address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    let response = client
        .post(&format!("{}/subscriptions", &address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // assert
    assert_eq!(500, response.status().as_u16());

    let saved = subscriptions
        .first::<Subscription>(&connection)
        .expect("Result set was empty.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // arrange
    let (address, _) = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // act
        let response = client
            .post(&format!("{}/subscriptions", &address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        // assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    // arrange
    let (address, _) = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        // act
        let response = client
            .post(&format!("{}/subscriptions", &address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request");

        // assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 200 OK when the payload was {}.",
            description
        )
    }
}

async fn spawn_app() -> (String, Settings) {
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
