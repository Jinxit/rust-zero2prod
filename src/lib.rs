#[macro_use]
extern crate rocket;

#[get("/health_check")]
async fn health_check() -> () {
    ()
}

pub async fn run() -> Result<(), rocket::Error> {
    rocket::build()
        .mount("/hello", routes![health_check])
        .launch()
        .await
}
