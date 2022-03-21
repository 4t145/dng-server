

mod logger;
mod error;
mod types;
mod consts;
// mod config;
mod room;

mod player;
mod observer;

/// # 协程
/// 1. 主：负责逻辑
/// 2. 推流：负责通讯
/// 
use tokio::net::{TcpListener};
// use tokio::sync::{oneshot, mpsc};
use crate::logger::log;
// use types::*;
// use std::env;
#[tokio::main]
async fn main() {
    console_subscriber::init();
    let launch_time = tokio::time::Instant::now();
    let mut handles = vec![];
    for port in 9000..9020 {
        let handle = tokio::spawn(
            run_on_port(port)
        );
        handles.push(handle);
    }
    let launch_finished_time = tokio::time::Instant::now();
    let duration = (launch_finished_time-launch_time).as_micros();
    log(format!("launch in {}ms", duration));

    for h in handles {
        let s = tokio::join!(h).0.unwrap_or(Ok(()));
        s.unwrap_or_default();
    }
}


async fn run_on_port(port: u16) -> Result<(), ()> {
    let try_socket = TcpListener::bind(format!("0.0.0.0:{}", port)).await;
    let listener = try_socket.expect("Failed to bind");
    log(format!("listening on port {}", port));
    let mut room = room::Room::new();
    let token_watcher = room.subscribe_token();
    let room_tx = room.get_tx();
    let _handle_room = tokio::spawn(async move {room.run().await});

    while let Ok((stream, _)) = listener.accept().await {
        let room_tx_clone = room_tx.clone();
        let token_watcher_clone = token_watcher.clone();
        tokio::spawn(async move {
            let (login_res_tx, login_res_rx) = tokio::sync::oneshot::channel::<LoginType>();
            let addr = stream.peer_addr().expect("connected streams should have a peer address");
            log(format!("Listening on: {}", addr));
            let ws_stream = tokio_tungstenite::accept_hdr_async(stream, Callback{
                login_result: login_res_tx,
                token_reciever: token_watcher_clone
            })
            .await
            .expect("Error during the websocket handshake occurred");
            
            match login_res_rx.await {
                Ok(LoginType::Player) => {
                    room_tx_clone.send(room::RoomReq::PlayerLogin{ws_stream}).await.unwrap();
                }
                Ok(LoginType::Observer(name)) => {
                    room_tx_clone.send(room::RoomReq::ObserverLogin { ws_stream, addr: addr.to_string(), name}).await.unwrap();
                },
                _ => {}
            }
        });
    }

    Ok(())
}

use tokio_tungstenite::tungstenite::handshake::server::Request as HsReq;
use tokio_tungstenite::tungstenite::handshake::server::Response as HsResp;
use tokio_tungstenite::tungstenite::handshake::server::ErrorResponse as HsError;
use tokio_tungstenite::tungstenite::handshake::server::Callback as HsCallback;

enum LoginType {
    Player,
    Observer(String),
    Reject,
}


struct Callback {
    token_reciever: tokio::sync::watch::Receiver<Option<u128>>,
    login_result: tokio::sync::oneshot::Sender<LoginType>
}

impl HsCallback for Callback {
    fn on_request(self, req: &HsReq, resp: HsResp ) -> Result<HsResp, HsError> {
        let path = req.uri().path();
        let error_info = if path == "/login" {
            let token = *self.token_reciever.borrow();
            match token {
                None => {
                    self.login_result.send(LoginType::Player).unwrap_or_default();
                    return Ok(resp);
                },
                Some(token) => {
                    if let Some(req_token) = req.headers().get("room-token") {
                        if let Ok(req_token)= req_token.to_str() {
                            if let Ok(req_token) = u128::from_str_radix(req_token, 16) {
                                if req_token == token {
                                    self.login_result.send(LoginType::Player).unwrap_or_default();
                                    return Ok(resp);
                                } else {"WrongToken"}
                            } else {"NotHex"}
                        } else {"CanNotDecode"}
                    } else {"NoToken"}
                }
            }
        } else if path == "/ob" {
            self.login_result.send(LoginType::Observer("Anon".to_string())).unwrap_or_default();
            return Ok(resp);
        } else {
            "PathError"
        };
        self.login_result.send(LoginType::Reject).unwrap_or_default();
        Err(HsError::new(Some(error_info.to_string())))
    }
}

