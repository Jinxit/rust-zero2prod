use zero2prod::configuration::get_configuration;
use zero2prod::email::SesEmailClient;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let email_client = SesEmailClient::new(&configuration).await;

    Application::build(&configuration, Box::new(email_client))
        .await?
        .server
        .launch()
        .await
}
