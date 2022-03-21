use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;
use serde::{Serialize, /* Deserialize */};

#[derive(Serialize)]
pub struct PortAndKey {
    port: u16,
    key: [u8;8]
}

#[derive(Clone)]
pub struct RoomMap(pub Arc<RwLock<HashMap<u32, PortAndKey>>>);
impl RoomMap {
    pub fn new() -> Self {
        RoomMap(
            Arc::new(
            RwLock::new(
                    HashMap::new()
                )
            )
        )
    }
}