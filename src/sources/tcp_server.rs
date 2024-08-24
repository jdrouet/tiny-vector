use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::Instrument;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("unable to parse address")]
    InvalidAddress(#[source] std::net::AddrParseError),
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct Config {
    pub address: Option<String>,
}

impl Config {
    pub fn build(self, sender: crate::prelude::Sender) -> Result<Source, BuildError> {
        let address = match self.address {
            Some(value) => value
                .parse::<SocketAddr>()
                .map_err(BuildError::InvalidAddress)?,
            None => SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 4000)),
        };
        Ok(Source { address, sender })
    }
}

async fn handle_connection(
    stream: TcpStream,
    sender: crate::prelude::Sender,
) -> std::io::Result<()> {
    stream.readable().await?;
    let mut reader = BufReader::new(stream);
    loop {
        let mut buffer = Vec::with_capacity(4096);
        match reader.read_until(b'\n', &mut buffer).await {
            Ok(0) => break,
            Ok(n) => match serde_json::from_slice::<crate::event::Event>(&buffer[..n]) {
                Ok(message) => {
                    if let Err(err) = sender.try_send(message) {
                        tracing::error!("unable to send message: {err:?}");
                    }
                }
                Err(err) => {
                    tracing::error!("invalid message received: {err:?}")
                }
            },
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

pub struct Source {
    address: SocketAddr,
    sender: crate::prelude::Sender,
}

impl Source {
    #[cfg(test)]
    fn new(address: SocketAddr, sender: crate::prelude::Sender) -> Self {
        Self { address, sender }
    }

    async fn iterate(&self, listener: &TcpListener) -> std::io::Result<()> {
        let (stream, address) = listener.accept().await?;
        let span = tracing::info_span!("connection", client = %address);
        let sender = self.sender.clone();
        tokio::spawn(async move {
            let _entered = span.enter();
            if let Err(err) = handle_connection(stream, sender).await {
                tracing::error!("connection failed: {err:?}");
            }
        });
        Ok(())
    }

    async fn execute(self, listener: TcpListener) {
        tracing::info!("waiting for connections");
        loop {
            if let Err(error) = self.iterate(&listener).await {
                tracing::error!("something went wrong: {error:?}");
            }
        }
    }

    pub async fn run(self, name: &str) -> tokio::task::JoinHandle<()> {
        let listener = TcpListener::bind(self.address).await.unwrap();

        let span = tracing::info_span!("component", name, kind = "source", flavor = "tcp_server");
        tokio::spawn(async move { self.execute(listener).instrument(span).await })
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::time::Duration;

    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;

    async fn wait_for(rx: &crate::prelude::Receiver) {
        for _ in 0..100 {
            if !rx.is_empty() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("no event received");
    }

    #[tokio::test]
    async fn should_receive_events() {
        crate::init_tracing();

        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5000);
        let (tx, rx) = crate::prelude::create_channel(10);
        let source = super::Source::new(address, tx);

        let _handle = source.run("name").await;

        let mut client = TcpStream::connect(address).await.unwrap();
        let event = crate::event::Event::Log(crate::event::EventLog::new("Hello World!"));
        let event_bytes = serde_json::to_vec(&event).unwrap();
        client.write(&event_bytes).await.unwrap();
        client.write("\n".as_bytes()).await.unwrap();

        wait_for(&rx).await;
    }

    #[tokio::test]
    async fn should_keep_going_after_failure() {
        crate::init_tracing();

        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5001);
        let (tx, rx) = crate::prelude::create_channel(10);
        let source = super::Source::new(address, tx);

        let _handle = source.run("name").await;

        let mut client = TcpStream::connect(address).await.unwrap();

        let event = crate::event::Event::Log(crate::event::EventLog::new("Hello World!"));
        let event_bytes = serde_json::to_vec(&event).unwrap();
        client.write(&event_bytes).await.unwrap();
        client.write("\n".as_bytes()).await.unwrap();

        client
            .write("this is not an event".as_bytes())
            .await
            .unwrap();
        client.write("\n".as_bytes()).await.unwrap();

        let event = crate::event::Event::Log(crate::event::EventLog::new("Hello World!"));
        let event_bytes = serde_json::to_vec(&event).unwrap();
        client.write(&event_bytes).await.unwrap();
        client.write("\n".as_bytes()).await.unwrap();

        wait_for(&rx).await;

        assert_eq!(rx.len(), 2);
    }
}
