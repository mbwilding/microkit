use crate::ServicePort;
use anyhow::{Result, anyhow};
use std::net::SocketAddr;
use tokio::net::{TcpListener, lookup_host};

pub async fn network(
    host: &Option<String>,
    port_base: ServicePort,
    port_offset: Option<u16>,
) -> Result<(SocketAddr, TcpListener)> {
    let host = match host {
        Some(host) => host,
        None => "0.0.0.0",
    };
    let port = match port_offset {
        Some(port_offset) => port_base.get_with_offset(port_offset),
        // This is used when hosting remotely for a predictable port
        None => 80,
    };
    let mut addrs = lookup_host((host, port)).await?;
    let address = addrs
        .find(|addr| addr.is_ipv4())
        .or_else(|| {
            let mut addrs = addrs;
            addrs.next()
        })
        .ok_or_else(|| anyhow!("Failed to look up host: {}:{}", host, port))?;
    let listener = TcpListener::bind(address).await?;
    let local_address = listener.local_addr()?;

    log::info!("{}: http://{}", port_base, local_address);

    Ok((local_address, listener))
}
