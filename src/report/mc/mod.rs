use std::{
    net::{SocketAddr, TcpStream},
    result,
    time::Duration,
};

use axum::{extract::Path, Json};
use serde::Serialize;

use crate::error::Error;

use self::packet::{
    handshake::Handshake,
    ping::{PingRequest, PingResponse},
    status::{StatusRequest, StatusResponse},
    Packet, SendPacket,
};

mod bytes;
mod packet;

type Result<T> = result::Result<T, Error>;

pub fn status() -> Result<StatusResponse> {
    let mut stream = create_tcp_stream()?;
    let stream = &mut stream;

    Handshake::default().send_packet(stream)?;

    StatusRequest.send_packet(stream)?;

    Packet::recv(stream)?.try_into()
}

pub fn ping(payload: i64) -> Result<PingResponse> {
    let mut stream = create_tcp_stream()?;
    let stream = &mut stream;

    Handshake::default().send_packet(stream)?;

    PingRequest { payload }.send_packet(stream)?;

    Packet::recv(stream)?.try_into().map_err(Error::Generic)
}

fn create_tcp_stream() -> Result<TcpStream> {
    let ip = std::env::var("MC_SERVER_ADDR")?.parse()?;
    let port = std::env::var("MC_SERVER_PORT")?.parse()?;

    let five_sec = Duration::from_secs(5);
    let stream = TcpStream::connect_timeout(&SocketAddr::new(ip, port), five_sec)?;
    stream.set_read_timeout(Some(five_sec))?;
    stream.set_write_timeout(Some(five_sec))?;

    Ok(stream)
}

#[derive(Serialize)]
pub struct Response<T> {
    status: String,
    response: Option<T>,
}

fn handle_route<F, T>(route_func: F) -> Json<Response<T>>
where
    F: FnOnce() -> Result<T>,
{
    let (status, response) = match route_func() {
        Ok(status_response) => ("ok".to_string(), Some(status_response)),
        Err(err) => {
            let error = err;
            (format!("{error:?}"), None)
        }
    };

    Json(Response { status, response })
}

pub async fn ping_route(Path(payload): Path<i64>) -> Json<Response<PingResponse>> {
    handle_route(|| ping(payload))
}

pub async fn status_route() -> Json<Response<StatusResponse>> {
    handle_route(status)
}
