use async_trait::async_trait;
use diesel::prelude::*;
use diesel::{Connection, PgConnection};
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use zero2prod::configuration::{get_configuration, Settings};
use zero2prod::domain::SubscriberEmail;
use zero2prod::email::Email;
use zero2prod::startup::Application;
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

pub struct TestApp {
    pub address: String,
    pub db_connection: PgConnection,
    pub email_client: Arc<MockEmailClient>,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }
}

pub struct SentEmail {
    pub recipient: SubscriberEmail,
    pub subject: String,
    pub html_content: String,
    pub text_content: String,
}

pub struct MockEmailClient {
    pub sent_emails: Mutex<Vec<SentEmail>>,
}

impl MockEmailClient {
    fn new() -> Self {
        Self {
            sent_emails: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl Email for MockEmailClient {
    async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> anyhow::Result<()> {
        Ok(self.sent_emails.lock().unwrap().push(SentEmail {
            recipient,
            subject: subject.to_string(),
            html_content: html_content.to_string(),
            text_content: text_content.to_string(),
        }))
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        c.application.port = None;
        c.database.database_name = Uuid::new_v4().to_string();
        println!("spawning with name {} ", c.database.database_name);
        c
    };

    let db_connection = setup_database(&configuration);

    let email_client = Arc::new(MockEmailClient::new());

    let app = Application::build(&configuration, email_client.clone())
        .await
        .unwrap();
    let _ = tokio::spawn(app.server.launch());
    TestApp {
        address: format!("http://127.0.0.1:{}", app.port.get().await),
        db_connection,
        email_client,
    }
}

fn setup_database(configuration: &Settings) -> PgConnection {
    let connection = connect_without_database(configuration);

    diesel::sql_query(format!(
        "CREATE DATABASE \"{}\"",
        configuration.database.database_name
    ))
    .execute(&connection)
    .unwrap();

    let connection = connect_to_database(configuration);

    diesel_migrations::run_pending_migrations(&connection).unwrap();
    connection
}

fn connect_to_database(configuration: &Settings) -> PgConnection {
    let connection_string = configuration.database.connection_string();
    let connection =
        PgConnection::establish(&connection_string).expect("Failed to connect to Postgres.");
    connection
}

fn connect_without_database(configuration: &Settings) -> PgConnection {
    let connection_string = configuration.database.connection_string_without_database();
    let connection =
        PgConnection::establish(&connection_string).expect("Failed to connect to Postgres.");
    connection
}
