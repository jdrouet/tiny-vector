#[derive(Clone, Debug, Default)]
pub struct Config {
    /// Interval between emitting events, in ms
    pub interval: Option<u64>,
}

fn generate() -> crate::event::Event {
    crate::event::EventLog::new("Hello World!").into()
}

impl Config {
    pub fn build(self, sender: crate::prelude::Sender) -> Source {
        Source {
            duration: tokio::time::Duration::from_millis(self.interval.unwrap_or(1000)),
            sender,
        }
    }
}

pub struct Source {
    duration: tokio::time::Duration,
    sender: crate::prelude::Sender,
}

impl Source {
    pub async fn execute(self) {
        let mut timer = tokio::time::interval(self.duration);
        loop {
            let _ = timer.tick().await;
            if let Err(_) = self.sender.try_send(generate()) {
                eprintln!("unable to send generated log");
            }
        }
    }

    pub fn run(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move { self.execute().await })
    }
}
