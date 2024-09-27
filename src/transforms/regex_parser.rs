use tracing::Instrument;

use crate::components::collector::Collector;
use crate::components::name::ComponentName;
use crate::components::output::ComponentWithOutputs;
use crate::event::log::EventLog;
use crate::event::Event;
use crate::prelude::Receiver;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("unable to compile pattern")]
    UnableToCompileRegex(
        #[from]
        #[source]
        regex::Error,
    ),
}

#[derive(Clone, Debug, serde::Deserialize)]
#[cfg_attr(test, derive(Default))]
pub struct Config {
    pattern: String,
}

impl ComponentWithOutputs for Config {}

impl Config {
    pub fn build(self) -> Result<Transform, BuildError> {
        Ok(Transform {
            pattern: regex::Regex::new(self.pattern.as_str())?,
        })
    }
}

pub struct Transform {
    pattern: regex::Regex,
}

impl Transform {
    fn handle_log(&self, event_log: EventLog) -> EventLog {
        let EventLog {
            mut attributes,
            message,
        } = event_log;
        let mut new_message = None::<String>;
        if let Some(capture) = self.pattern.captures(&message) {
            eprintln!("matching");
            for name in self.pattern.capture_names() {
                if let Some(name) = name {
                    match capture.name(name) {
                        Some(value) if name == "message" => {
                            new_message = Some(value.as_str().to_owned());
                        }
                        Some(value) => {
                            attributes
                                .insert(name.to_owned().into(), value.as_str().to_owned().into());
                        }
                        None => {}
                    }
                }
            }
            EventLog {
                attributes,
                message: new_message.unwrap_or(message),
            }
        } else {
            eprintln!("nothing matching");
            EventLog {
                attributes,
                message,
            }
        }
    }

    fn handle(&self, event: Event) -> Event {
        match event {
            Event::Log(inner) => Event::Log(self.handle_log(inner)),
            Event::Metric(inner) => Event::Metric(inner),
        }
    }

    async fn execute(self, mut receiver: Receiver, collector: Collector) {
        tracing::info!("starting");
        while let Some(event) = receiver.recv().await {
            if let Err(err) = collector.send_default(self.handle(event)).await {
                tracing::error!("unable to send generated log: {err:?}");
            }
        }
        tracing::info!("stopping");
    }

    pub async fn run(
        self,
        name: &ComponentName,
        receiver: Receiver,
        collector: Collector,
    ) -> tokio::task::JoinHandle<()> {
        let span = tracing::info_span!(
            "component",
            name = name.as_ref(),
            kind = "transform",
            flavor = "add_fields"
        );
        tokio::spawn(async move { self.execute(receiver, collector).instrument(span).await })
    }
}
#[cfg(test)]
mod tests {
    use crate::components::collector::Collector;
    use crate::components::name::ComponentName;
    use crate::components::output::NamedOutput;
    use crate::event::metric::EventMetricValue;
    use crate::prelude::create_channel;

    #[tokio::test]
    async fn should_extract_from_logs() {
        let config = super::Config {
            pattern: String::from(
                r"^service=(?<service>[a-z]+)\s+status=(?<status>[a-z]+)\s+(?<message>.*)$",
            ),
        };
        let transform = config.build().unwrap();
        let (output_tx, mut output_rx) = create_channel(10);
        let (input_tx, input_rx) = create_channel(10);

        input_tx
            .send(
                crate::event::metric::EventMetric::new(
                    crate::helper::now(),
                    "foo",
                    "bar",
                    EventMetricValue::Gauge(42.0),
                )
                .with_tag("hostname", "fake-server")
                .into(),
            )
            .await
            .unwrap();
        input_tx
            .send(
                crate::event::log::EventLog::new("service=something status=ok hello world").into(),
            )
            .await
            .unwrap();
        input_tx
            .send(crate::event::log::EventLog::new("whatever status=ok hello world").into())
            .await
            .unwrap();

        let collector = Collector::default().with_output(NamedOutput::Default, output_tx);
        let handler = transform
            .run(&ComponentName::new("transform"), input_rx, collector)
            .await;

        let event_metric = output_rx.recv().await.unwrap().into_event_metric().unwrap();
        assert_eq!(event_metric.value, EventMetricValue::Gauge(42.0));

        let event_log = output_rx.recv().await.unwrap().into_event_log().unwrap();
        assert_eq!(event_log.message, "hello world");
        assert_eq!(
            event_log
                .attributes
                .get("service")
                .and_then(|v| v.as_text())
                .unwrap(),
            "something"
        );
        assert_eq!(
            event_log
                .attributes
                .get("status")
                .and_then(|v| v.as_text())
                .unwrap(),
            "ok"
        );

        let event_log = output_rx.recv().await.unwrap().into_event_log().unwrap();
        assert_eq!(event_log.message, "whatever status=ok hello world");

        handler.abort();
    }
}
