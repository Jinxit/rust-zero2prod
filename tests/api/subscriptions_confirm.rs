use crate::helpers::spawn_app;
use claim::assert_some;

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    // arrange
    let app = spawn_app().await;

    // act
    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    // assert
    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    // arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    app.post_subscriptions(body.into()).await;

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

    // act
    let response = reqwest::get(confirmation_links.html).await.unwrap();

    // assert
    assert_eq!(response.status().as_u16(), 200);
}
