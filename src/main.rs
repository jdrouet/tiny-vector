mod event;
mod sinks;
mod sources;
mod topology;

#[tokio::main]
async fn main() {
    let topo = topology::Config::default()
        .with_source(
            "demo-logs",
            "console",
            crate::sources::Config::RandomLogs(Default::default()),
        )
        .with_sink("console", crate::sinks::Config::Console(Default::default()))
        .build();
    topo.run().wait().await;
}
