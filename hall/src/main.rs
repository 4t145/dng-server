use std::sync::Arc;

use tokio::sync::RwLock;
use warp::{Filter, reject, reply::Json};
use roommap::RoomMap;
use manifest::{Manifest, ManifestResp};

use clap::Parser;

// use std::env;
// use log::{info, warn};

mod roommap;
mod manifest;

// FROM 9000 TO 9100
const PUB_PORT:u16 = 9000;
const PUB_SIZE:u16 = 20;
// const ROOM_SERVER_IP:&str = "127.0.0.1";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    roomip: String,
    #[clap(short, long)]
    port: u16,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    println!("try to observe ports");
    let mut manifest = Manifest::from_ports(args.roomip, PUB_PORT..PUB_PORT+PUB_SIZE);
    manifest.ob_all().await;
    let manifest_resp = ManifestResp::from(&manifest);
    let manifest = manifest.rwlock();
    // let locked_manifest = manifest_data.get_lock();
    // let locked_manifest_view = manifest_data.get_manifest();
    println!("initial http services");
    

    let cors = warp::cors().allow_any_origin().allow_methods(vec!["GET", "POST", "DELETE"]);


    let room_state_service=
    warp::path!("state"/ String)
    .and(warp::any().map(move||manifest.clone()))
    .and_then(room_state_handle).with(cors.clone());


    let jump_map = RoomMap::new();
    let room_jump_service = 
    warp::path!("room" / String)
    .and(warp::any().map(move||jump_map.clone()))
    .and_then(room_jump_handle);


    let manifest_service = 
    warp::path!("manifest")
    .and(warp::any().map(move||manifest_resp.clone()))
    .and_then(manifest_handle).with(cors.clone());

    

    warp::serve(room_jump_service.or(room_state_service).or(manifest_service)).run(([0,0,0,0], args.port)).await;
}


async fn room_jump_handle(hexcode:String, room_map:RoomMap) -> Result<Json, reject::Rejection> {
    if let Ok(key) = u32::from_str_radix(hexcode.as_str(), 16) {
        let hashtable = room_map.0.read().await;
        if let Some(reply) = hashtable.get(&key) {
            let json_reply = warp::reply::json(reply);
            return Ok(json_reply);
        }
    }
    Err(reject())
}

async fn room_state_handle(room_id: String, manifest_data: Arc<RwLock<Manifest>>) -> Result<Json, reject::Rejection> {
    {
        let manifest_unlocked = manifest_data.read().await;
        if let Some(room) = manifest_unlocked.rooms.get(&room_id) {
            if let Some(ref conn) = room.connection {
                let state = conn.watcher.borrow().clone();
                let json_reply = warp::reply::json(&state);
                return Ok(json_reply);
            }
        }
    }

    Err(reject())
}

async fn manifest_handle(manifest_resp: ManifestResp) -> Result<Json, reject::Rejection> {
    let json_reply = warp::reply::json(&manifest_resp);
    return Ok(json_reply);
}