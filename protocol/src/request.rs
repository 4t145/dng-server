// use bincode::{serialize ,deserialize};
use serde::{Deserialize, Serialize};
use crate::BinCodeMessage;

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub enum PlayerRequest {
    SetName {
        name: String,
    }, 
    Chat {
        msg: String,
    },
    ImReady,
    ImUnready,
    Frame {
        bin: Vec<u8>
    },
    Mark {
        score: i8,
    },
    Lexicon(Vec<String>),
    LexiconService(u32),
}

impl BinCodeMessage<'_> for PlayerRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObserverRequest {
    
}
impl BinCodeMessage<'_> for ObserverRequest {}