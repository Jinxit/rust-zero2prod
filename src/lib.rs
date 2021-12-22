#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_sync_db_pools;
#[macro_use]
extern crate diesel;

pub mod catchers;
pub mod configuration;
pub mod models;
pub mod port_saver;
pub mod routes;
pub mod schema;
pub mod startup;
