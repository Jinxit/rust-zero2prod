use zero2prod::run;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    run().await
}
