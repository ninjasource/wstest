use core::panic;
use embedded_websocket as ws;
use embedded_websocket::WebSocketOptions;
use rand::rngs::ThreadRng;

use std::io::{Read, Write};
use std::net::TcpStream;
use thiserror::Error;
use ws::{WebSocketClient, WebSocketReceiveMessageType, WebSocketSendMessageType};

extern crate native_tls;
use native_tls::TlsConnector;

#[derive(Error, Debug)]
pub enum Error {
    #[error("embedded_websocket error: {0:?}")]
    EmbeddedWebsocket(ws::Error),
    #[error("payload too large to fit in buffer: {0:?} bytes")]
    PayloadTooLarge(usize),
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

fn write_all<T: Write>(stream: &mut T, buffer: &[u8]) -> Result<(), std::io::Error> {
    let mut from = 0;
    loop {
        let bytes_sent = stream.write(&buffer[from..])?;
        from = from + bytes_sent;

        if from == buffer.len() {
            break;
        }
    }

    stream.flush()?;
    Ok(())
}

pub fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
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

    let mut buf: [u8; 4096] = [0; 4096];
    // heap allocated memory to store payload for one entire websocket frame
    let mut output_json = vec![0; 1024 * 1024];

    let mut ws_client = ws::WebSocketClient::new_client(rand::thread_rng());

    // initiate a websocket opening handshake
    let websocket_options = WebSocketOptions {
        path: "/",
        host: "ws-feed.pro.coinbase.com",
        origin: "ws-feed.pro.coinbase.com",
        sub_protocols: None,
        additional_headers: None,
    };

    let (len, web_socket_key) = ws_client
        .client_connect(&websocket_options, &mut buf)
        .map_err(Error::EmbeddedWebsocket)?;
    println!("Sending opening handshake: {} bytes", len);
    stream.write_all(&buf[..len])?;

    // read the response from the server and check it to complete the opening handshake
    let received_size = stream.read(&mut buf)?;
    ws_client
        .client_accept(&web_socket_key, &mut buf[..received_size])
        .map_err(Error::EmbeddedWebsocket)?;
    println!("Opening handshake completed successfully");

    let send_size = ws_client
        .write(
            WebSocketSendMessageType::Text,
            true,
            &TEST_SUBSCRIPTION.as_bytes(),
            &mut buf,
        )
        .map_err(Error::EmbeddedWebsocket)?;

    println!("Sending {} bytes", send_size);
    write_all(&mut stream, &buf[..send_size])?;
    let mut write_cursor = 0;

    loop {
        // read the response from the server (we expect the server to simply echo the same message back)
        let received_size = stream.read(&mut buf)?;
        let mut read_cursor = 0;

        loop {
            if read_cursor == received_size {
                break;
            }

            if write_cursor == output_json.len() {
                return Err(Error::PayloadTooLarge(output_json.len()))?;
            }

            let ws_result = ws_client
                .read(
                    &buf[read_cursor..received_size],
                    &mut output_json[write_cursor..],
                )
                .unwrap();
            read_cursor += ws_result.len_from;

            match ws_result.message_type {
                WebSocketReceiveMessageType::Text => {
                    write_cursor += ws_result.len_to;

                    if ws_result.end_of_message {
                        let _s = std::str::from_utf8(&output_json[..write_cursor])?;
                        println!("{}", _s);
                        //println!("Text received of {} characters received", s.len());
                        write_cursor = 0;
                    }
                }
                x => println!("Enexpected {:?} message type received", x),
            }
        }
    }
}
/*
struct FrameReader<'a, TStream>
where
    TStream: Read,
{
    write_cursor: usize,
    read_cursor: usize,
    stream: &'a mut TStream,
    ws_client: &'a mut WebSocketClient<ThreadRng>,
    buf: &'a mut [u8],
    frame_payload: &'a mut [u8],
}

impl<'a, TStream> FrameReader<'a, TStream>
where
    TStream: Read,
{
    fn new(
        stream: &'a mut TStream,
        ws_client: &'a mut ws::WebSocketClient<ThreadRng>,
        buf: &'a mut [u8],
        frame_payload: &'a mut [u8],
    ) -> Self {
        Self {
            write_cursor: 0,
            read_cursor: 0,
            stream,
            ws_client,
            buf,
            frame_payload,
        }
    }
}

impl<'a, TStream> Iterator for FrameReader<'a, TStream>
where
    TStream: Read,
{
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        loop {
            let received_size = self.stream.read(&mut self.buf).unwrap();
            self.read_cursor = 0;

            loop {
                if self.read_cursor == received_size {
                    break;
                }

                if self.write_cursor == self.frame_payload.len() {
                    panic!("payload too large");
                }

                let ws_result = self
                    .ws_client
                    .read(
                        &self.buf[self.read_cursor..received_size],
                        &mut self.frame_payload[self.write_cursor..],
                    )
                    .unwrap();
                self.read_cursor += ws_result.len_from;

                match ws_result.message_type {
                    WebSocketReceiveMessageType::Text => {
                        self.write_cursor += ws_result.len_to;

                        if ws_result.end_of_message {
                            let slice = &self.frame_payload[..self.write_cursor];
                            self.write_cursor = 0;
                            return Some(slice);
                        }
                    }
                    WebSocketReceiveMessageType::CloseCompleted => return None,
                    WebSocketReceiveMessageType::CloseMustReply => return None,
                    x => println!("Enexpected {:?} message type received", x),
                }
            }
        }
    }
}
*/
