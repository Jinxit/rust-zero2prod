use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use async_trait::async_trait;
use diesel::prelude::*;
use diesel::{Connection, PgConnection};
use once_cell::sync::Lazy;
use reqwest::Url;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use zero2prod::configuration::{get_configuration, Settings};
use zero2prod::domain::SubscriberEmail;
use zero2prod::email::Email;
use zero2prod::models::NewUser;
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
    pub port: u16,
    pub address: String,
    pub db_connection: PgConnection,
    pub email_client: Arc<MockEmailClient>,
    pub test_user: TestUser,
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

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/newsletters", &self.address))
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub fn get_confirmation_links(&self, email: &SentEmail) -> ConfirmationLinks {
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();

            let mut confirmation_link = Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(&email.html_content);
        let plain_text = get_link(&email.text_content);
        ConfirmationLinks { html, plain_text }
    }
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    fn store(&self, conn: &PgConnection) {
        use zero2prod::schema::users;
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::default()
            .hash_password(self.password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        diesel::insert_into(users::table)
            .values(NewUser {
                user_id: &self.user_id,
                username: &self.username,
                password_hash: &password_hash,
            })
            .execute(conn)
            .expect("Failed to store test user.");
    }
}

pub struct SentEmail {
    pub recipient: String,
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
        recipient: &SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), anyhow::Error> {
        Ok(self.sent_emails.lock().unwrap().push(SentEmail {
            recipient: recipient.as_ref().to_string(),
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
    let port = app.port.get().await;

    let test_user = TestUser::generate();

    test_user.store(&db_connection);

    TestApp {
        port,
        address: format!("http://127.0.0.1:{}", port),
        db_connection,
        email_client,
        test_user,
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
