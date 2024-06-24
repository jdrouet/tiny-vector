#[derive(Clone, Debug, Default)]
pub struct Config {}

pub struct Sink {
    receiver: crate::prelude::Receiver,
}

impl Config {
    pub fn build(self) -> (Sink, crate::prelude::Sender) {
        let (sender, receiver) = crate::prelude::create_channel(1000);
        (Sink { receiver }, sender)
    }
}

impl Sink {
    async fn execute(mut self) {
        while let Some(input) = self.receiver.recv().await {
            println!("{input:?}");
        }
    }

    pub fn run(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move { self.execute().await })
    }
}
