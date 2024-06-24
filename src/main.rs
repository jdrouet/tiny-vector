mod event;
mod prelude;
mod sinks;
mod sources;
mod topology;

#[tokio::main]
async fn main() {
    let config = crate::topology::Config::from_path("./example.toml").unwrap();
    let topo = config.build().unwrap();
    topo.run().wait().await;
}
