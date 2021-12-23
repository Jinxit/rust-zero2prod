use crate::catchers::*;
use crate::configuration::Settings;
use crate::port_saver;
use crate::port_saver::Port;
use crate::routes::*;
use rocket::figment::{
    util::map,
    value::{Map, Value},
};
use rocket::{Config, Ignite, Rocket};
use rocket_sync_db_pools::{database, diesel};

#[database("newsletter")]
pub struct NewsletterDbConn(diesel::PgConnection);

pub async fn build(settings: &Settings) -> Result<(Rocket<Ignite>, Port), rocket::Error> {
    let (port_saver, port) = port_saver::create_pair();
    let db: Map<_, Value> = map! {
        "url" => settings.database.connection_string().into()
    };
    rocket::build()
        .configure(
            Config::figment()
                .merge(("databases", map!["newsletter" => db]))
                .merge(Config {
                    port: settings.application_port.unwrap_or(0),
                    ..Config::debug_default()
                }),
        )
        .attach(port_saver)
        .attach(NewsletterDbConn::fairing())
        .mount("/", routes![health, subscribe])
        .register("/", catchers![unprocessable_entity_to_bad_request])
        .ignite()
        .await
        .map(|rocket| (rocket, port))
}
