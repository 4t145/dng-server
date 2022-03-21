use crate::consts::*;
use crate::types::*;
use crate::room::RoomReq;
use protocol::PlayerResponse;
use tokio::sync::{mpsc};
use tokio::task::JoinHandle;
use tokio::time;
use futures::{StreamExt, SinkExt};


use protocol::{BinCodeMessage, PlayerRequest, PlayerState};

#[derive(Debug)]
pub struct Player {
    pub(crate) ready: bool,
    pub(crate) name: String,
    pub(crate) right: [bool; ROOM_SIZE],
    pub(crate) score: [u8;3],
    pub(crate) timepoint: u16,
    pub(crate) drawpoint: u8,

    pub(crate) ws_from_room_tx: mpsc::Sender<WsMsg>,

    pub(crate) _tx_handle: JoinHandle<()>,
    pub(crate) _rx_handle: JoinHandle<()>,
    pub(crate) _ping_handle: JoinHandle<()>,
}

impl Player {

    pub(crate) fn new(idx: usize, stream: WsStream, room_tx: mpsc::Sender<RoomReq>) -> Self {

        let (mut ws_tx, mut ws_rx) = stream.split();
        let (ws_from_room_tx, mut ws_from_room_rx) = mpsc::channel::<WsMsg>(128);

        let tx_ping = ws_from_room_tx.clone();
        let _ping_handle = tokio::spawn(async move {
            loop {
                tx_ping.send(WsMsg::Ping(Vec::new())).await.unwrap_or_default();
                time::sleep(HB_DURATION).await;
            }
        });

        let room_transmit_tx = room_tx.clone();
        let _rx_handle = tokio::spawn(
            async move {
                // let mut last_ping = time::Instant::now();
                while let Some(Ok(ws_msg)) = ws_rx.next().await {
                    // let now = time::Instant::now();
                    match ws_msg {
                        WsMsg::Text(_) => {},
                        WsMsg::Binary(bin) => {
                            // last_ping = time::Instant::now();
                            if let Ok(req) = PlayerRequest::deser(&bin) {
                                room_transmit_tx.send(RoomReq::PlayerReq(idx, req)).await.unwrap_or_default();
                            }
                        },
                        WsMsg::Close(_) =>{
                            room_transmit_tx.send(RoomReq::PlayerLogout(idx)).await.unwrap_or_default();
                            
                            break;
                        },
                        WsMsg::Pong(_) => {
                            // last_ping = time::Instant::now();
                            
                        }
                        WsMsg::Ping(_) => {},
                        WsMsg::Frame(_) => {},
                    }

                    // if now-last_ping > HB_TIMEOUT {
                    //     room_transmit_tx.send(RoomReq::PlayerLogout(idx)).await.unwrap_or_default();
                    //     break;
                    // }
                }
            }
        );

        let logout_reminder = room_tx.clone();
        let _tx_handle = tokio::spawn(
            async move {
                use tokio_tungstenite::tungstenite::error::Error::AlreadyClosed 
                as AlreadyClosed;
                while let Some(ws_msg) = ws_from_room_rx.recv().await {
                    match ws_tx.send(ws_msg).await {
                        Err(AlreadyClosed) => {
                            logout_reminder.send(RoomReq::PlayerLogout(idx)).await.unwrap_or_default();
                            break;
                        }
                        _ => {},
                    };
                }
            }
        );

        Self {
            ready: false,
            name: format!("<{}>", idx),
            right: [false; ROOM_SIZE],
            score: [0;3],
            timepoint: 0,
            drawpoint: 0,

            ws_from_room_tx,

            _tx_handle,
            _rx_handle,
            _ping_handle
        }
    }

    pub(crate) async fn send(&self, resp: PlayerResponse) {
        if let Ok(msg) = resp.ser() {
            self.ws_from_room_tx.send(msg).await.unwrap_or_default()
        }
    }
    pub(crate) fn _abort(&self) {
        self._rx_handle.abort();
        self._tx_handle.abort();
        self._ping_handle.abort();
    }
    
    pub(crate) fn ready(&mut self) {self.ready = true;}
    pub(crate) fn unready(&mut self) {self.ready = false;}
    pub(crate) fn set_right(&mut self, idx: usize) {self.right[idx] = true;}
    pub(crate) fn add_draw_point(&mut self) {self.drawpoint+=1}
    pub(crate) fn get_state(&self, idx:usize) -> PlayerState {
        PlayerState {
            name: self.name.clone(),
            ready: self.ready,
            idx: idx as u8,
            score: self.score,
            timepoint: self.timepoint,
            drawpoint: self.drawpoint
        }
    }

    pub(crate) fn set_name(&mut self, name: impl ToString) {
        self.name = name.to_string();
    }

    pub(crate) fn get_name(&self) -> String {
        self.name.clone()
    }

    pub(crate) fn mark(&mut self, score: i8) {
        let idx:usize = match score {
            -1 => 0,
             0 => 1,
             1 => 2,
             _ => 1
        };
        self.score[idx] += 1;
    }

    pub(crate) fn reset(&mut self) {
        self.ready = false;
        self.right = [false; ROOM_SIZE];
        self.score = [0;3];
        self.timepoint = 0;
        self.drawpoint = 0;
    }
}


