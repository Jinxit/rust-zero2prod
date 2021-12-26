use crate::catchers::*;
use crate::configuration::Settings;
use crate::port_saver;
use crate::port_saver::Port;
use crate::routes::*;
use rocket::fairing::Fairing;
use rocket::figment::{
    util::map,
    value::{Map, Value},
};
use rocket::{Config, Ignite, Rocket};
use rocket_sync_db_pools::{database, diesel, ConnectionPool};

#[database("newsletter")]
pub struct NewsletterDbConn(diesel::PgConnection);

impl NewsletterDbConn {
    pub fn named_fairing(database_name: String) -> impl Fairing {
        let pool_name = Box::leak(Box::new(format!("'{}' Database Pool", database_name)));
        let database_name = Box::leak(Box::new(database_name));

        <ConnectionPool<Self, diesel::PgConnection>>::fairing(pool_name, database_name)
    }
}

pub async fn build(settings: &Settings) -> Result<(Rocket<Ignite>, Port), rocket::Error> {
    let (port_saver, port) = port_saver::create_pair();
    let db: Map<_, Value> = map! {
        "url" => settings.database.connection_string().into()
    };
    rocket::build()
        .configure(
            Config::figment()
                .merge((
                    "databases",
                    map![settings.database.database_name.clone() => db],
                ))
                .merge(Config {
                    port: settings.application_port.unwrap_or(0),
                    ..Config::debug_default()
                }),
        )
        .attach(port_saver)
        .attach(NewsletterDbConn::named_fairing(
            settings.database.database_name.clone(),
        ))
        .mount("/", routes![health, subscribe])
        .register("/", catchers![unprocessable_entity_to_bad_request])
        .ignite()
        .await
        .map(|rocket| (rocket, port))
}