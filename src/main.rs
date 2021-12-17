#[macro_use] extern crate rocket;

#[get("/health_check")]
async fn health_check() -> () {
    ()
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![
            health_check
        ])
}