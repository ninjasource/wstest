use framer::{Framer, FramerError};
use embedded_websocket::{WebSocketCloseStatusCode, WebSocketOptions, WebSocketSendMessageType};
use std::net::TcpStream;
use thiserror::Error;

extern crate native_tls;
use native_tls::TlsConnector;
mod framer;

#[derive(Error, Debug)]
pub enum MainError {
    #[error("frame reader error: {0:?}")]
    FrameReader(FramerError),
    #[error("io error: {0:?}")]
    Io(std::io::Error),
}

impl From<FramerError> for MainError {
    fn from(err: FramerError) -> Self {
        MainError::FrameReader(err)
    }
}

impl From<std::io::Error> for MainError {
    fn from(err: std::io::Error) -> Self {
        MainError::Io(err)
    }
}

const TEST_SUBSCRIPTION: &'static str = r#"
{
    "type": "subscribe",
    "channels": [
        {
            "name": "level2",
            "product_ids": [
                "ETH-BTC"
            ]
        },
        {
            "name": "level2",
            "product_ids": [
                "ETH-USD"
            ]
        }
    ]
}
"#;

pub fn main() -> Result<(), MainError> {
    //    let url = Url::parse("wss://ws-feed-public.sandbox.pro.coinbase.com").unwrap();
    //    let url = Url::parse("wss://ws-feed.pro.coinbase.com").unwrap();

    let address = "ws-feed.pro.coinbase.com:443";
    println!("Connecting to: {}", address);
    println!("Connected.");

    let connector = TlsConnector::new().unwrap();

    let stream = TcpStream::connect(address).unwrap();
    let mut stream = connector
        .connect("ws-feed.pro.coinbase.com", stream)
        .unwrap();

    let mut read_buf: [u8; 4096] = [0; 4096];
    let mut write_buf: [u8; 4096] = [0; 4096];

    // heap allocated memory to store payload for one entire websocket frame
    let mut frame_buf = vec![0; 1024 * 1024];

    let mut ws_client = embedded_websocket::WebSocketClient::new_client(rand::thread_rng());

    // initiate a websocket opening handshake
    let websocket_options = WebSocketOptions {
        path: "/",
        host: "ws-feed.pro.coinbase.com",
        origin: "ws-feed.pro.coinbase.com",
        sub_protocols: None,
        additional_headers: None,
    };

    let mut websocket = Framer::new(&mut read_buf, &mut write_buf, &mut ws_client, &mut stream);
    websocket.connect(&websocket_options)?;

    websocket.write(
        WebSocketSendMessageType::Text,
        true,
        &TEST_SUBSCRIPTION.as_bytes(),
    )?;

    while let Some(s) = websocket.read_text(&mut frame_buf)? {
        println!("{}", s);
    }

    websocket.close(WebSocketCloseStatusCode::NormalClosure, None)?;
    return Ok(());
}
