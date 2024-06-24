pub mod console;

#[derive(Clone, Debug)]
pub enum Config {
    Console(self::console::Config),
}

impl Config {
    pub fn build(self) -> (Sink, crate::prelude::Sender) {
        match self {
            Self::Console(inner) => {
                let (inner, tx) = inner.build();
                (Sink::Console(inner), tx)
            }
        }
    }
}

pub enum Sink {
    Console(self::console::Sink),
}

impl Sink {
    pub fn run(self) -> tokio::task::JoinHandle<()> {
        match self {
            Self::Console(inner) => inner.run(),
        }
    }
}
