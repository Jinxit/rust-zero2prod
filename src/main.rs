use zero2prod::configuration::get_configuration;
use zero2prod::startup::build;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let configuration = get_configuration().expect("Failed to read configuration.");

    build(&configuration).await?.0.launch().await
}
