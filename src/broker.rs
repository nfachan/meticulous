mod heap;
mod scheduler;

use crate::{proto, Result};

/// The main function for the client. This should be called on a task of its own. It will return
/// when a signal is received or when all work has been processed by the broker.
pub async fn main(port: Option<u16>) -> Result<()> {
    let sockaddr =
        std::net::SocketAddrV6::new(std::net::Ipv6Addr::UNSPECIFIED, port.unwrap_or(0), 0, 0);
    let listener = tokio::net::TcpListener::bind(sockaddr).await?;
    println!("listening on: {}", listener.local_addr()?);

    loop {
        let (mut socket, _) = listener.accept().await?;
        let hello: proto::Hello = proto::read_message(&mut socket).await?;
        println!("got: {hello:?}");
        std::mem::forget(socket);
    }
}