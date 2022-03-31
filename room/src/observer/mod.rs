

use crate::types::*;
use crate::consts::*;
use crate::room::RoomReq;
use protocol::ObserverRequest;
use protocol::ObserverResponse;
use tokio::sync::{mpsc};
use futures::{StreamExt, SinkExt};

use protocol::{BinCodeMessage};
use tokio::task::JoinHandle;
use tokio::time;

#[derive(Debug)]
pub struct Observer {
    pub(crate) localname: String,

    pub(crate) ws_from_room_tx: mpsc::Sender<WsMsg>,

    pub(crate) _tx_handle: JoinHandle<()>,
    pub(crate) _rx_handle: JoinHandle<()>,
    pub(crate) _ping_handle: JoinHandle<()>,
}

impl Observer {
    pub(crate) fn new(name: String, addr:String, stream: WsStream, room_tx: mpsc::Sender<RoomReq>) -> Self {
        
        let localname = format!("{}[{}]", addr, name);
        let localname_clone = localname.clone();
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
                while let Some(Ok(ws_msg)) = ws_rx.next().await {
                    match ws_msg {
                        WsMsg::Text(_) => {},
                        WsMsg::Binary(bin) => {
                            if let Ok(req) = ObserverRequest::deser(&bin) {
                                room_transmit_tx.send(RoomReq::ObserverReq(localname_clone.clone(), req)).await.unwrap_or_default();
                            }
                        },
                        WsMsg::Close(_) =>{
                            room_transmit_tx.send(RoomReq::ObserverLogout(localname_clone)).await.unwrap_or_default();
                            
                            break;
                        },
                        _ => {}
                    }
                }
                
            }
        );

        let logout_reminder = room_tx.clone();
        let localname_clone = localname.clone();
        let _tx_handle = tokio::spawn(
            async move {
                use tokio_tungstenite::tungstenite::error::Error::AlreadyClosed 
                as AlreadyClosed;
                while let Some(ws_msg) = ws_from_room_rx.recv().await {
                    match ws_tx.send(ws_msg).await {
                        Err(AlreadyClosed) => {
                            logout_reminder.send(RoomReq::ObserverLogout(localname_clone)).await.unwrap_or_default();
                            break;
                        }
                        _ => {},
                    };
                }
            }
        );

        Self {
            localname,

            ws_from_room_tx,

            _tx_handle,
            _rx_handle,
            _ping_handle,
        }
    }

    pub(crate) async fn send(&self, resp: ObserverResponse) {
        if let Ok(msg) = resp.ser() {
            self.ws_from_room_tx.send(msg).await.unwrap_or_default()
        }
    }

    pub(crate) fn _abort(&self) {
        self._rx_handle.abort();
        self._tx_handle.abort();
    }

}