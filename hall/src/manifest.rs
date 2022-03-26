use std::collections::{ HashMap};
use protocol::RoomState;
use tokio::task::JoinHandle;

// use async_trait::async_trait;
use futures_util::{StreamExt, /* SinkExt */};
use tokio::sync::RwLock;
use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::{Message}};
use serde::{Serialize};

use tokio::sync::watch;

use protocol::*;

pub struct RoomConnection {
    pub handle: JoinHandle<()>,
    pub watcher: tokio::sync::watch::Receiver<Option<RoomState>>
}

pub struct RoomItem {
    pub(crate) url: String,
    pub(crate) name: Option<String>,
    pub(crate) connection: Option<RoomConnection>,
}

pub struct Manifest {
    pub rooms: HashMap<String, RoomItem>
}

#[derive(Clone, Serialize)]
pub struct ManifestRespItem {
    id: String,
    url: String,
    name: Option<String>
}

#[derive(Default, Clone, Serialize)]
pub struct ManifestResp(Vec<ManifestRespItem>);
impl ManifestResp {
    pub fn from(manifest: &Manifest) -> Self {
        let mut v = vec![];
        for (id, room) in &manifest.rooms {
            v.push(ManifestRespItem {
                id: id.clone(),
                url: room.url.clone(),
                name: room.name.clone(),
            })
        }
        ManifestResp(v)
    }
}


impl RoomItem {
    pub async fn ob(&mut self) -> Result<(), ()> {
        let url = format!("ws://{}/ob", self.url);
        if let Ok((ws_stream, _resp)) = connect_async(url).await {
            let (mut _tx, mut rx) = ws_stream.split();
            let (reposter, watcher) = watch::channel(None);
            let handle = tokio::spawn(
                async move {
                    use tokio::time::{Instant, Duration};
                    let mut last_ping = Instant::now();
                    while let Some(Ok(inbound)) = rx.next().await {
                        let now = Instant::now();
                        match inbound {
                            Message::Text(_) => {},
                            Message::Binary(bin) => {
                                if let Ok(ObserverResponse::RoomState(room_state)) = ObserverResponse::deser(&bin) {
                                    reposter.send(Some(room_state)).unwrap_or_default();
                                }
                            },
                            Message::Ping(_) => {
                                last_ping = now;
                            },
                            Message::Pong(_) => {},
                            Message::Close(_) => {
                                break;
                            },
                            Message::Frame(_) => {},
                        }

                        if now - last_ping > Duration::from_secs(60) {

                            break;
                        }
                    }
                    reposter.send(None).unwrap_or_default();
                }
            );
            let conn = RoomConnection {
                handle,
                watcher,
            };
            self.connection = Some(conn);
            return Ok(())
            
        }
        return Err(())
    }
}



impl Manifest {
    pub fn from_ports(ip:impl std::fmt::Display, ports: impl Iterator<Item = u16>) -> Self {
        let mut id:usize = 0;
        let mut rooms = HashMap::new();
        for port in ports {
            let url = format!("{}:{}", ip, port);
            let key = id.to_string();

            rooms.insert(key, RoomItem {
                url,
                name: None,
                connection: None,
            });

            id+=1;
        }

        Self {
            rooms
        }
    }


    pub async fn ob_all(&mut self) {

        for (_id, room) in self.rooms.iter_mut() {
            if let Ok(_) = room.ob().await {

            }
        }
    }

    pub fn rwlock<'l>(self) -> Arc<RwLock<Self>> {
        let lock = Arc::new(RwLock::new(self));
        return lock
    }

}

