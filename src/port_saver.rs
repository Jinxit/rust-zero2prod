use rocket::fairing::Info;
use rocket::{Orbit, Rocket};
use std::sync::Mutex;
use tokio::sync::mpsc;

pub fn create_pair() -> (PortSaver, Port) {
    let (tx, rx) = mpsc::channel(1);
    let port_saver = PortSaver::new(tx);
    let port = Port::new(rx);
    (port_saver, port)
}

pub struct Port {
    port: Mutex<Option<u16>>,
    rx: Mutex<mpsc::Receiver<u16>>,
}

impl Port {
    fn new(rx: mpsc::Receiver<u16>) -> Port {
        Port {
            port: Mutex::new(None),
            rx: Mutex::new(rx),
        }
    }

    pub async fn get(&self) -> u16 {
        let mut port_guard = self.port.lock().unwrap();
        match *port_guard {
            Some(port) => port,
            None => {
                let mut rx_guard = self.rx.lock().unwrap();
                let port = rx_guard.recv().await.unwrap();
                *port_guard = Some(port);
                port
            }
        }
    }
}

pub struct PortSaver {
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
