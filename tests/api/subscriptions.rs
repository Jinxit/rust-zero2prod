use crate::helpers::spawn_app;
use diesel::{Connection, PgConnection, RunQueryDsl};
use zero2prod::models::*;
use zero2prod::schema::subscriptions::dsl::*;

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
