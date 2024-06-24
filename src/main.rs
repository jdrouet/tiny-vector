mod event;
mod prelude;
mod sinks;
mod sources;
mod topology;

#[tokio::main]
async fn main() {
    let topo = topology::Config::default()
        .with_source(
            "demo-logs",
            crate::sources::Config::RandomLogs(Default::default()),
        )
        .with_sink(
            "console",
            ["demo-logs"],
            crate::sinks::Config::Console(Default::default()),
        )
        .build();
    topo.run().wait().await;
}
