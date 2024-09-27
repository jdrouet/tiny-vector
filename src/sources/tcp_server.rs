use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::Instrument;

use crate::components::collector::Collector;
use crate::components::output::ComponentWithOutputs;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("unable to parse address")]
    InvalidAddress(#[source] std::net::AddrParseError),
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct Config {
    pub address: Option<String>,
}

impl ComponentWithOutputs for Config {}

impl Config {
    pub fn build(self) -> Result<Source, BuildError> {
        let address = match self.address {
            Some(value) => value
                .parse::<SocketAddr>()
                .map_err(BuildError::InvalidAddress)?,
            None => SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 4000)),
        };
        Ok(Source {
            state: Stale { address },
        })
    }
}

async fn handle_connection(stream: TcpStream, collector: Collector) -> std::io::Result<()> {
    stream.readable().await?;
    let mut reader = BufReader::new(stream);
    loop {
        let mut buffer = Vec::with_capacity(4096);
        match reader.read_until(b'\n', &mut buffer).await {
            Ok(0) => break,
            Ok(n) => match serde_json::from_slice::<crate::event::Event>(&buffer[..n]) {
                Ok(message) => {
                    if let Err(err) = collector.send_default(message).await {
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

#[derive(Debug, thiserror::Error)]
pub(crate) enum StartingError {
    #[error("unable to bind socket")]
    UnableToBind(#[source] std::io::Error),
}

pub(crate) struct Stale {
    address: SocketAddr,
}

pub(crate) struct Running {
    listener: TcpListener,
}

pub struct Source<S = Stale> {
    state: S,
}

impl<S> Source<S> {
    pub const fn flavor(&self) -> &'static str {
        "tcp_server"
    }
}

impl Source<Stale> {
    #[cfg(test)]
    fn new(address: SocketAddr) -> Self {
        Self {
            state: Stale { address },
        }
    }

    async fn prepare(self) -> Result<Source<Running>, StartingError> {
        let listener = TcpListener::bind(self.state.address)
            .await
            .map_err(StartingError::UnableToBind)?;

        Ok(Source {
            state: Running { listener },
        })
    }

    pub async fn run(
        self,
        span: tracing::Span,
        collector: Collector,
    ) -> Result<tokio::task::JoinHandle<()>, StartingError> {
        let prepared = self.prepare().await?;
        Ok(tokio::spawn(async move {
            prepared.execute(collector).instrument(span).await
        }))
    }
}

impl Source<Running> {
    async fn iterate(&self, collector: Collector) -> std::io::Result<()> {
        let (stream, address) = self.state.listener.accept().await?;
        let span = tracing::info_span!("connection", client = %address);
        tokio::spawn(async move {
            let _entered = span.enter();
            if let Err(err) = handle_connection(stream, collector).await {
                tracing::error!("connection failed: {err:?}");
            }
        });
        Ok(())
    }

    async fn execute(self, collector: Collector) {
        tracing::info!("waiting for connections");
        loop {
            if let Err(error) = self.iterate(collector.clone()).await {
                tracing::error!("something went wrong: {error:?}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::time::Duration;

    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;

    use crate::components::collector::Collector;
    use crate::components::output::NamedOutput;

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
        let collector = Collector::default().with_output(NamedOutput::Default, tx);
        let source = super::Source::new(address);

        let _handle = source.run(tracing::info_span!("foo"), collector).await;

        let mut client = TcpStream::connect(address).await.unwrap();
        let event = crate::event::Event::Log(crate::event::log::EventLog::new("Hello World!"));
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
        let collector = Collector::default().with_output(NamedOutput::Default, tx);
        let source = super::Source::new(address);

        let _handle = source.run(tracing::info_span!("foo"), collector).await;

        let mut client = TcpStream::connect(address).await.unwrap();

        let event = crate::event::Event::Log(crate::event::log::EventLog::new("Hello World!"));
        let event_bytes = serde_json::to_vec(&event).unwrap();
        client.write(&event_bytes).await.unwrap();
        client.write("\n".as_bytes()).await.unwrap();

        client
            .write("this is not an event".as_bytes())
            .await
            .unwrap();
        client.write("\n".as_bytes()).await.unwrap();

        let event = crate::event::Event::Log(crate::event::log::EventLog::new("Hello World!"));
        let event_bytes = serde_json::to_vec(&event).unwrap();
        client.write(&event_bytes).await.unwrap();
        client.write("\n".as_bytes()).await.unwrap();

        wait_for(&rx).await;

        assert_eq!(rx.len(), 2);
    }
}
