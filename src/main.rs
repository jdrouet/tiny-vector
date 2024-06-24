mod event;
mod prelude;
mod sinks;
mod sources;
mod topology;

#[tokio::main]
async fn main() {
    let config = crate::topology::Config::from_path("./example.toml");
    let topo = config.build();
    topo.run().wait().await;
}
