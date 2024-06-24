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
            "output",
            ["demo-logs"],
            crate::sinks::Config::DatadogLog(
                crate::sinks::datadog_log::Config::default()
                    .with_base_url("https://http-intake.logs.datadoghq.eu/api/v2/logs")
                    .with_api_token(""),
            ),
        )
        .build();
    topo.run().wait().await;
}
