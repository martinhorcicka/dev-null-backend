use axum::Json;

use super::A2sInfo;

fn a2s_info() -> Json<A2sInfo> {
    let ip_addr = std::env::var("SE_SERVER_ADDR").unwrap().parse().unwrap();
    let port = std::env::var("SE_SERVER_PORT").unwrap().parse().unwrap();

    super::a2s_info(ip_addr, port)
}
