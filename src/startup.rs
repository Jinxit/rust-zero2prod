use crate::catchers::*;
use crate::configuration::Settings;
use crate::diesel::Connection;
use crate::email::Email;
use crate::port_saver;
use crate::port_saver::Port;
use crate::routes::*;
use diesel::PgConnection;
use rocket::fairing::Fairing;
use rocket::figment::{
    util::map,
    value::{Map, Value},
};
use rocket::{Config, Ignite, Rocket};
use rocket_sync_db_pools::{database, diesel, ConnectionPool};
use std::sync::Arc;

pub struct Application {
    pub port: Port,
    pub server: Rocket<Ignite>,
}

impl Application {
    pub async fn build(
        settings: &Settings,
        email_client: Arc<dyn Email>,
    ) -> Result<Self, rocket::Error> {
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
                        port: settings.application.port.unwrap_or(0),
                        address: settings.application.host,
                        ..Config::default()
                    }),
            )
            .attach(port_saver)
            .attach(NewsletterDbConn::named_fairing(
                settings.database.database_name.clone(),
            ))
            .manage(email_client)
            .manage(ApplicationBaseUrl(settings.application.base_url.clone()))
            .mount("/", routes![health, subscribe, confirm])
            .register("/", catchers![unprocessable_entity_to_bad_request])
            .ignite()
            .await
            .map(|server| Application { port, server })
    }
}

pub struct ApplicationBaseUrl(pub String);

#[database("newsletter")]
pub struct NewsletterDbConn(diesel::PgConnection);

impl NewsletterDbConn {
    pub fn named_fairing(database_name: String) -> impl Fairing {
        let pool_name = Box::leak(Box::new(format!("'{}' Database Pool", database_name)));
        let database_name = Box::leak(Box::new(database_name));

        <ConnectionPool<Self, diesel::PgConnection>>::fairing(pool_name, database_name)
    }

    pub async fn run_transaction<T, E, F, G>(&self, f: F, error_mapper: G) -> Result<T, E>
    where
        T: Send + 'static,
        E: Send + 'static,
        F: FnOnce(&diesel::PgConnection) -> Result<T, E> + Send + 'static,
        G: FnOnce(diesel::result::Error) -> E + Send + 'static,
    {
        self.run(move |c: &mut PgConnection| {
            let mut closure_error: Option<E> = None;
            c.transaction(|| {
                f(&c).map_err(|e| {
                    closure_error = Some(e);
                    diesel::result::Error::RollbackTransaction
                })
            })
            .map_err(|diesel_error| closure_error.unwrap_or_else(|| error_mapper(diesel_error)))
        })
        .await
    }
}
