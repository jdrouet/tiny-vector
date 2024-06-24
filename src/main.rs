mod event;
mod prelude;
mod sinks;
mod sources;
mod topology;

#[tokio::main]
async fn main() {
    if let Err(err) = tracing_subscriber::fmt().try_init() {
        eprintln!("unable to init tracing: {err:?}");
    }

    let config = crate::topology::Config::from_path("./example.toml").unwrap();
    let topo = config.build().unwrap();
    topo.run().wait().await;
}
