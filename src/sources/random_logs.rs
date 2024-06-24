#[derive(Debug)]
pub struct BuildError;

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct Config {
    /// Interval between emitting events, in ms
    pub interval: Option<u64>,
}

fn generate() -> crate::event::Event {
    crate::event::EventLog::new("Hello World!")
        .with_attribute("hostname", "fake-server")
        .with_attribute("ddsource", "tiny-vector")
        .into()
}

impl Config {
    pub fn build(self, sender: crate::prelude::Sender) -> Result<Source, BuildError> {
        Ok(Source {
            duration: tokio::time::Duration::from_millis(self.interval.unwrap_or(1000)),
            sender,
        })
    }
}

pub struct Source {
    duration: tokio::time::Duration,
    sender: crate::prelude::Sender,
}

impl Source {
    pub async fn execute(self) {
        tracing::info!("starting random_logs source execution");
        let mut timer = tokio::time::interval(self.duration);
        loop {
            let _ = timer.tick().await;
            tracing::debug!("generating new random log");
            if let Err(err) = self.sender.try_send(generate()) {
                tracing::error!("unable to send generated log: {err:?}");
                break;
            }
        }
        tracing::info!("stopping random_logs source execution");
    }

    pub fn run(self, name: &str) -> tokio::task::JoinHandle<()> {
        let span = tracing::info_span!("component", name, kind = "source", flavor = "random_logs");
        tokio::spawn(async move {
            let _entered = span.enter();
            self.execute().await
        })
    }
}
