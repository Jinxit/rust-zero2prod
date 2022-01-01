use crate::domain::SubscriberEmail;
use serde;
use serde_aux::field_attributes::deserialize_number_from_string;
use serde_aux::field_attributes::deserialize_option_number_from_string;
use std::net::IpAddr;

pub enum Environment {
    Local,
    Production,
}

#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub email_client: EmailClientSettings,
}

#[derive(serde::Deserialize)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_option_number_from_string")]
    pub port: Option<u16>,
    pub host: IpAddr,
    pub base_url: String,
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

#[derive(serde::Deserialize)]
pub struct EmailClientSettings {
    pub sender_email: String,
    pub timeout_milliseconds: u64,
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either 'local' or 'production'.",
                other
            )),
        }
    }
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}?sslmode={}",
            self.username,
            self.password,
            self.host,
            self.port,
            self.database_name,
            ssl_mode(self.require_ssl)
        )
    }

    pub fn connection_string_without_database(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}?sslmode={}",
            self.username,
            self.password,
            self.host,
            self.port,
            ssl_mode(self.require_ssl)
        )
    }
}

fn ssl_mode(require_ssl: bool) -> &'static str {
    match require_ssl {
        true => "require",
        false => "prefer",
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("configuration");
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT");

    let mut settings = config::Config::default();
    settings.merge(config::File::from(configuration_directory.join("base")).required(true))?;
    settings.merge(
        config::File::from(configuration_directory.join(environment.as_str())).required(true),
    )?;
    settings.merge(config::Environment::with_prefix("app").separator("__"))?;
    settings.try_into()
}
