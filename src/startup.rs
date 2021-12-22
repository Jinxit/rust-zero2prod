use crate::catchers::*;
use crate::port_saver;
use crate::port_saver::Port;
use crate::routes::*;
use rocket::{Config, Ignite, Rocket};

pub async fn build(server_port: Option<u16>) -> Result<(Rocket<Ignite>, Port), rocket::Error> {
    let (port_saver, port) = port_saver::create_pair();
    rocket::custom(Config {
        port: server_port.unwrap_or(0),
        ..Config::debug_default()
    })
    .attach(port_saver)
    .mount("/", routes![health, subscribe])
    .register("/", catchers![unprocessable_entity_to_bad_request])
    .ignite()
    .await
    .map(|rocket| (rocket, port))
}
