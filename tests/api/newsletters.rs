use crate::helpers::{spawn_app, ConfirmationLinks, TestApp};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // arrange
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    // act

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .json(&newsletter_request_body)
        .send()
        .await
        .expect("Failed to execute request.");

    // assert
    assert_eq!(response.status().as_u16(), 200);
    let emails = app.email_client.sent_emails.lock().unwrap();
    assert_eq!(
        emails.len(),
        0,
        "Expected 0 emails, {} were sent",
        emails.len()
    );
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    // act

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .json(&newsletter_request_body)
        .send()
        .await
        .expect("Failed to execute request.");

    // assert
    assert_eq!(response.status().as_u16(), 200);
    let emails = app.email_client.sent_emails.lock().unwrap();
    assert_eq!(
        emails.len(),
        1,
        "Expected 1 email, {} were sent",
        emails.len()
    );
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    let mut emails = app.email_client.sent_emails.lock().unwrap();
    app.get_confirmation_links(&emails.pop().unwrap())
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(app).await;

    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
