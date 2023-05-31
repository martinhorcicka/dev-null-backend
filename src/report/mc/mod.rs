use std::{
    net::{SocketAddr, TcpStream},
    result,
    time::Duration,
};

use crate::{error::Error, response::Response};

use self::packet::{
    handshake::Handshake,
    ping::{PingRequest, PingResponse},
    status::{StatusRequest, StatusResponse},
    Packet, SendPacket,
};

mod bytes;
pub mod packet;

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

    Packet::recv(stream)?.try_into()
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
fn handle_route<F, T>(route_func: F) -> Response<T>
where
    F: FnOnce() -> Result<T>,
{
    match route_func() {
        Ok(val) => Response::ok(val),
        Err(err) => Response::err(err),
    }
}

pub fn ping_route(payload: i64) -> Response<PingResponse> {
    handle_route(|| ping(payload))
}

pub fn status_route() -> Response<StatusResponse> {
    handle_route(status)
}
