pub mod request;
pub mod response;
pub mod error;
pub mod lexicon;

pub use response::*;
pub use request::*;
pub use lexicon::Lexicon;

use serde::{Serialize, Deserialize};
use bincode::{serialize, deserialize, ErrorKind};
use tokio_tungstenite::tungstenite::Message;

pub trait BinCodeMessage<'a>: Serialize + Deserialize<'a>{
    fn deser(bin: &'a [u8]) -> Result<Self, Box<ErrorKind>> {
        deserialize::<Self>(&bin)
    }

    fn ser(&self) -> Result<Message, Box<ErrorKind>> {
        let bin = serialize(&self)?;
        Ok(Message::Binary(bin))
    }
}
