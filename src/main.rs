mod event;
mod sinks;
mod sources;

#[tokio::main]
async fn main() {
    let (source_task, source_rx) =
        crate::sources::random_logs::Source::new(crate::sources::random_logs::Config::default());
    let sink_task = crate::sinks::console::Sink::new(Default::default(), source_rx);
    //
    let sink_handler = sink_task.run();
    let source_handler = source_task.run();
    //
    source_handler.await.expect("source failed");
    sink_handler.await.expect("sink failed");
}
