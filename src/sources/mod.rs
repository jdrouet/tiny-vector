pub mod random_logs;

#[derive(Clone, Debug)]
pub enum Config {
    RandomLogs(self::random_logs::Config),
}

impl Config {
    pub fn build(self) -> (Source, tokio::sync::mpsc::Receiver<crate::event::Event>) {
        match self {
            Self::RandomLogs(inner) => {
                let (source, rx) = inner.build();
                (Source::RandomLogs(source), rx)
            }
        }
    }
}

pub enum Source {
    RandomLogs(self::random_logs::Source),
}

impl Source {
    pub fn run(self) -> tokio::task::JoinHandle<()> {
        match self {
            Self::RandomLogs(inner) => inner.run(),
        }
    }
}
