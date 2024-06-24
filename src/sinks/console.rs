#[derive(Clone, Debug, Default)]
pub struct Config {}

pub struct Sink {
    reader: tokio::sync::mpsc::Receiver<crate::event::Event>,
}

impl Config {
    pub fn build(self, reader: tokio::sync::mpsc::Receiver<crate::event::Event>) -> Sink {
        Sink { reader }
    }
}

impl Sink {
    pub fn new(_config: Config, reader: tokio::sync::mpsc::Receiver<crate::event::Event>) -> Self {
        Self { reader }
    }

    async fn execute(mut self) {
        while let Some(input) = self.reader.recv().await {
            println!("{input:?}");
        }
    }

    pub fn run(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move { self.execute().await })
    }
}
