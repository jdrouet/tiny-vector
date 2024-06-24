#[derive(Debug, Default)]
pub struct Config {
    /// Interval between emitting events, in ms
    pub interval: Option<u64>,
}

fn generate() -> crate::event::Event {
    crate::event::EventLog::new("Hello World!").into()
}

pub struct Source {
    duration: tokio::time::Duration,
    output: tokio::sync::mpsc::Sender<crate::event::Event>,
}

impl Source {
    pub fn new(config: Config) -> (Self, tokio::sync::mpsc::Receiver<crate::event::Event>) {
        let (tx, rx) = tokio::sync::mpsc::channel::<crate::event::Event>(100);

        (
            Self {
                duration: tokio::time::Duration::from_millis(config.interval.unwrap_or(1000)),
                output: tx,
            },
            rx,
        )
    }

    pub async fn execute(self) {
        let mut timer = tokio::time::interval(self.duration);
        loop {
            let _ = timer.tick().await;
            if let Err(_) = self.output.try_send(generate()) {
                eprintln!("unable to send generated log");
            }
        }
    }

    pub fn run(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move { self.execute().await })
    }
}
