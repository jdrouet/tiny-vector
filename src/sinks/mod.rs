pub mod console;

#[derive(Clone, Debug)]
pub enum Config {
    Console(self::console::Config),
}

impl Config {
    pub fn build(self, reader: tokio::sync::mpsc::Receiver<crate::event::Event>) -> Sink {
        match self {
            Self::Console(inner) => Sink::Console(inner.build(reader)),
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
