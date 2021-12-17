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

async fn spawn_app() -> String {
    let (app, mut port) = zero2prod::build(Some(0)).await.unwrap();
    //let client = rocket::local::blocking::Client::tracked(app).unwrap();
    let _ = tokio::spawn(app.launch());
    format!("http://127.0.0.1:{}", port.get().await)
}
