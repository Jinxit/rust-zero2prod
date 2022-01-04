use crate::helpers::spawn_app;
use claim::assert_some;
use diesel::RunQueryDsl;
use zero2prod::models::*;
use zero2prod::schema::subscriptions::dsl::subscriptions;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // arrange
    let app = spawn_app().await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // act
    let response = app.post_subscriptions(body.into()).await;

    // assert
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    // arrange
    let app = spawn_app().await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // act
    app.post_subscriptions(body.into()).await;

    // assert
    let saved = subscriptions
        .first::<Subscription>(&app.db_connection)
        .expect("Result set was empty.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_returns_a_500_for_duplicate_email() {
    // arrange
    let app = spawn_app().await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // act
    app.post_subscriptions(body.into()).await;
    let response = app.post_subscriptions(body.into()).await;

    // assert
    assert_eq!(500, response.status().as_u16());

    let saved = subscriptions
        .first::<Subscription>(&app.db_connection)
        .expect("Result set was empty.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // arrange
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // act
        let response = app.post_subscriptions(invalid_body.into()).await;

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
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        // act
        let response = app.post_subscriptions(body.into()).await;

        // assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            description
        )
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    // arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // act
    app.post_subscriptions(body.into()).await;

    // assert
    let emails = app.email_client.sent_emails.lock().unwrap();
    assert_eq!(
        emails.len(),
        1,
        "Expected 1 email, {} were sent",
        emails.len()
    );
    let email = emails.get(0);
    assert_some!(email);
    let email = email.unwrap();
    assert_eq!(email.recipient, "ursula_le_guin@gmail.com");
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    // arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // act
    app.post_subscriptions(body.into()).await;

    // assert
    let emails = app.email_client.sent_emails.lock().unwrap();
    assert_eq!(
        emails.len(),
        1,
        "Expected 1 email, {} were sent",
        emails.len()
    );
    let email = emails.get(0);
    assert_some!(email);
    let email = email.unwrap();

    let confirmation_links = app.get_confirmation_links(&email);
    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    // arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // sabotage!
    diesel::sql_query("ALTER TABLE subscriptions DROP COLUMN email;")
        .execute(&app.db_connection)
        .unwrap();

    // act
    let response = app.post_subscriptions(body.into()).await;

    // assert
    assert_eq!(response.status().as_u16(), 500);
}
