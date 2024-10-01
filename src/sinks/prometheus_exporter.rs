use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use metrics::{Label, Recorder};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusRecorder};

use crate::event::metric::{EventMetric, EventMetricHeader, EventMetricValue};
use crate::event::Event;
use crate::prelude::Receiver;

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    address: Option<String>,
    bucket_duration: Option<u64>,
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("unable to parse address")]
    InvalidAddress(#[source] std::net::AddrParseError),
    #[error("unable to build bucket")]
    InvalidBucket(#[source] metrics_exporter_prometheus::BuildError),
}

impl Config {
    pub fn build(self) -> Result<Sink, BuildError> {
        let address = match self.address {
            Some(value) => value
                .parse::<SocketAddr>()
                .map_err(BuildError::InvalidAddress)?,
            None => SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9598)),
        };
        let bucket_duration = std::time::Duration::from_secs(self.bucket_duration.unwrap_or(60));
        let builder = PrometheusBuilder::new();
        let builder = builder
            .with_http_listener(address)
            .set_bucket_duration(bucket_duration)
            .map_err(BuildError::InvalidBucket)?;
        Ok(Sink {
            state: Stale { builder },
            metadata: metrics::Metadata::new("", metrics::Level::TRACE, None),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StartingError {
    #[error("unable to build recorder")]
    UnableToBuildRecorder(#[source] metrics_exporter_prometheus::BuildError),
}

pub(crate) struct Stale {
    builder: PrometheusBuilder,
}

pub(crate) struct Running {
    recorder: PrometheusRecorder,
    #[allow(dead_code)]
    exporter: tokio::task::JoinHandle<()>,
}

pub struct Sink<S = Stale> {
    state: S,
    metadata: metrics::Metadata<'static>,
}

impl<S> Sink<S> {
    pub(crate) fn flavor(&self) -> &'static str {
        "prometheus_exporter"
    }
}

impl super::Preparable for Sink<Stale> {
    type Output = Sink<Running>;
    type Error = StartingError;

    async fn prepare(self) -> Result<Self::Output, Self::Error> {
        let (recorder, exporter) = self
            .state
            .builder
            .build()
            .map_err(StartingError::UnableToBuildRecorder)?;

        let exporter = tokio::spawn(async move {
            if exporter.await.is_err() {
                tracing::error!("exporter failed");
            }
        });

        Ok(Sink {
            state: Running { recorder, exporter },
            metadata: self.metadata,
        })
    }
}

impl Sink<Running> {
    fn handle_metric(&mut self, event_metric: EventMetric) {
        let EventMetric {
            timestamp: _,
            header,
            value,
        } = event_metric;
        let EventMetricHeader { name, tags } = header;
        let labels = tags
            .into_iter()
            .map(|(key, value)| Label::new(key, value))
            .collect::<Vec<_>>();
        let key = metrics::Key::from_parts(name.to_string(), labels);
        match value {
            EventMetricValue::Gauge(inner) => {
                self.state
                    .recorder
                    .register_gauge(&key, &self.metadata)
                    .set(inner);
            }
            EventMetricValue::Counter(inner) => {
                self.state
                    .recorder
                    .register_counter(&key, &self.metadata)
                    .increment(inner);
            }
        }
    }

    fn handle(&mut self, event: Event) {
        match event {
            Event::Log(_) => {}
            Event::Metric(inner) => self.handle_metric(inner),
        }
    }
}

impl super::Executable for Sink<Running> {
    async fn execute(mut self, mut receiver: Receiver) {
        tracing::info!("starting");
        while let Some(input) = receiver.recv().await {
            self.handle(input);
        }
        tracing::info!("stopping");
    }
}
