use crate::types::*;

#[derive(/* Clone, Copy, */ Debug)]
pub enum TimerType {
    Draw,
    Mark,
}

#[derive(/* Clone, Copy, */ Debug)]
pub enum Request {
    PlayerReq(usize, protocol::PlayerRequest),
    PlayerLogin{
        ws_stream: WsStream,
    },
    PlayerLogout(usize),

    ObserverReq(String, protocol::ObserverRequest),
    ObserverLogin{
        addr: String,
        name: String,
        ws_stream: WsStream,
    },
    ObserverLogout(String),

    Timer(TimerType),
    CountDown(usize),
}