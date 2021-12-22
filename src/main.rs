use zero2prod::startup::build;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    build(Some(8000)).await?.0.launch().await
}
