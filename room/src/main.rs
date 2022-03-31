

mod logger;
mod error;
mod types;
mod consts;
// mod config;
mod room;

mod player;
mod observer;


use tokio::net::{TcpListener};
use tokio::task::JoinSet;
use crate::logger::log;
use crate::consts::*;




#[tokio::main]
async fn main() {
    // console_subscriber::init();

    
    let mut handles = JoinSet::new();
    for port in PORT_FROM..PORT_TO {
        handles.spawn(run_on_port(port));
    }
    
    loop {
        match handles.join_one().await {
            Ok(res) => {
                log(format!("{:?}", res));
            },
            Err(e) => {
                log(format!("join error {:?}", e));
            },
        }
        if handles.is_empty() {break};
    } 
}


#[derive(Debug)]
enum ErrorKind {
    Io(std::io::Error),
    Handshake(tokio_tungstenite::tungstenite::Error)
}

impl Drop for ErrorKind {
    fn drop(&mut self) {
        log(format!("error {:?}", self))
    }
}
async fn run_on_port(port: u16) -> Result<u16, ()> {
    let try_socket = TcpListener::bind(format!("0.0.0.0:{}", port)).await;
    let listener = try_socket.expect("Failed to bind");
    log(format!("listening on port {}", port));
    let name = format!("零号机-{}", port);
    let mut room = room::Room::new(Some(name));
    let token_watcher = room.subscribe_token();
    let room_tx = room.get_tx();
    let handle_room = tokio::spawn(async move {room.run().await});

    while let Ok((stream, _)) = listener.accept().await {
        let room_tx_clone = room_tx.clone();
        let token_watcher_clone = token_watcher.clone();
        tokio::spawn(async move {
            let (login_res_tx, login_res_rx) = tokio::sync::oneshot::channel::<LoginType>();
            let addr = stream.peer_addr()
                .map_err(ErrorKind::Io)?;
            log(format!("new connection on: {}", addr));
            let ws_stream = tokio_tungstenite::accept_hdr_async(stream, Callback{
                login_result: login_res_tx,
                token_reciever: token_watcher_clone
            })
            .await.map_err(ErrorKind::Handshake)?;
            
            
            match login_res_rx.await {
                Ok(LoginType::Player) => {
                    room_tx_clone.send(room::RoomReq::PlayerLogin{ws_stream}).await.unwrap_or_default();
                }
                Ok(LoginType::Observer(name)) => {
                    room_tx_clone.send(room::RoomReq::ObserverLogin { ws_stream, addr: addr.to_string(), name}).await.unwrap_or_default();
                },
                _ => {}
            }

            Result::<(), ErrorKind>::Ok(())
        });
    }

    handle_room.await.unwrap_or_default();
    Ok(port)
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

