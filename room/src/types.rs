
use tokio_tungstenite::WebSocketStream;
use tokio::net::TcpStream;
pub use tokio_tungstenite::tungstenite::{Message as WsMsg};



pub type WsStream = WebSocketStream<TcpStream>;
// pub type WsTx = SplitSink<WebSocketStream<TcpStream>, WsMsg>;
// pub type WsRx = SplitStream<WebSocketStream<TcpStream>>;