#[macro_use]
extern crate rocket;

use rocket::fairing::Info;
use rocket::{Config, Ignite, Orbit, Rocket};

use tokio::sync::mpsc;

pub struct Port {
    port: Option<u16>,
    rx: mpsc::Receiver<u16>,
}

impl Port {
    fn new(rx: mpsc::Receiver<u16>) -> Port {
        Port { port: None, rx }
    }

    pub async fn get(&mut self) -> u16 {
        match self.port {
            Some(port) => port,
            None => {
                let port = self.rx.recv().await.unwrap();
                self.port = Some(port);
                port
            }
        }
    }
}

struct PortSaver {
    sender: mpsc::Sender<u16>,
}

impl PortSaver {
    fn new(sender: mpsc::Sender<u16>) -> PortSaver {
        PortSaver { sender }
    }
}

#[rocket::async_trait]
impl rocket::fairing::Fairing for PortSaver {
    fn info(&self) -> Info {
        Info {
            name: "Port Saver",
            kind: rocket::fairing::Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        self.sender.send(rocket.config().port).await.unwrap();
    }
}

#[get("/health_check")]
async fn health_check() {}

pub async fn build(port: Option<u16>) -> Result<(Rocket<Ignite>, Port), rocket::Error> {
    let (tx, rx) = mpsc::channel(1);
    let port_saver = PortSaver::new(tx);
    rocket::custom(Config {
        port: port.unwrap_or(0),
        ..Config::debug_default()
    })
    .attach(port_saver)
    .mount("/", routes![health_check])
    .ignite()
    .await
    .map(|rocket| (rocket, Port::new(rx)))
}
