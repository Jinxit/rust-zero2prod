use diesel::prelude::*;
use diesel::{Connection, PgConnection};
use zero2prod::configuration::get_configuration;
use zero2prod::models::*;
use zero2prod::schema::subscriptions::dsl::*;
use zero2prod::schema::subscriptions::star;

#[tokio::test]
async fn health_check_works() {
    // arrange
    let address = spawn_app().await;

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
    let address = spawn_app().await;

    let configuration = get_configuration().expect("Failed to read configuration");

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
async fn subscribe_returns_a_400_when_data_is_missing() {
    // arrange
    let address = spawn_app().await;
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

async fn spawn_app() -> String {
    let (app, mut port) = zero2prod::startup::build(Some(0)).await.unwrap();
    //let client = rocket::local::blocking::Client::tracked(app).unwrap();
    let _ = tokio::spawn(app.launch());
    format!("http://127.0.0.1:{}", port.get().await)
}
