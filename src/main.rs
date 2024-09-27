mod components;
mod event;
mod helper;
mod prelude;
mod sinks;
mod sources;
mod topology;
mod transforms;

fn init_tracing() {
    if let Err(err) = tracing_subscriber::fmt().try_init() {
        eprintln!("unable to init tracing: {err:?}");
    }
}

#[tokio::main]
async fn main() {
    init_tracing();

    let config = crate::topology::Config::from_path("./example.toml").unwrap();
    let topo = config.build().await.unwrap();
    topo.start().await.unwrap().wait().await;
}
