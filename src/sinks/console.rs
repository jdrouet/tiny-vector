#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Config {}

#[derive(Debug)]
pub struct BuildError;

impl Config {
    pub fn build(self) -> Result<(Sink, crate::prelude::Sender), BuildError> {
        let (sender, receiver) = crate::prelude::create_channel(1000);
        Ok((Sink { receiver }, sender))
    }
}

pub struct Sink {
    receiver: crate::prelude::Receiver,
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
