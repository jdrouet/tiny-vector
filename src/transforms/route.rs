use indexmap::IndexMap;
use tokio::sync::mpsc::error::SendError;

use super::condition::prelude::Evaluate;
use super::condition::Condition;
use crate::components::collector::Collector;
use crate::components::output::{ComponentWithOutputs, NamedOutput};
use crate::event::Event;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error(transparent)]
    ConditionFailed(super::condition::BuildError),
    #[error("the fallback route {name} is conflicting with the defined routes")]
    FallbackRouteConflict { name: NamedOutput },
}

fn default_fallback() -> NamedOutput {
    NamedOutput::Named("dropped".into())
}

#[derive(Clone, Debug, serde::Deserialize)]
#[cfg_attr(test, derive(Default))]
pub struct Config {
    routes: IndexMap<NamedOutput, crate::transforms::condition::Config>,
    /// Route being used when no route condition is matching.
    /// The default being the "dropped" route.
    fallback: Option<NamedOutput>,
}

impl Config {
    fn is_fallback(&self, output: &NamedOutput) -> bool {
        if let Some(ref named) = self.fallback {
            named.eq(output)
        } else {
            default_fallback().eq(output)
        }
    }
}

impl ComponentWithOutputs for Config {
    fn has_output(&self, output: &NamedOutput) -> bool {
        self.routes.keys().any(|name| (*name).eq(output)) || self.is_fallback(output)
    }
}

impl Config {
    pub fn build(self) -> Result<Transform, BuildError> {
        let fallback = self.fallback.unwrap_or_else(default_fallback);
        if self.routes.contains_key(&fallback) {
            Err(BuildError::FallbackRouteConflict { name: fallback })
        } else {
            Ok(Transform {
                routes: self
                    .routes
                    .into_iter()
                    .map(|(name, condition)| {
                        condition
                            .build()
                            .map(|cond| (cond, name))
                            .map_err(BuildError::ConditionFailed)
                    })
                    .collect::<Result<_, BuildError>>()?,
                fallback,
            })
        }
    }
}

#[derive(Debug)]
pub struct Transform {
    routes: Vec<(Condition, NamedOutput)>,
    fallback: NamedOutput,
}

impl Transform {
    pub(crate) fn flavor(&self) -> &'static str {
        "route"
    }
}

impl super::Executable for Transform {
    fn handle(
        &self,
        collector: &Collector,
        event: Event,
    ) -> impl std::future::Future<Output = Result<(), SendError<Event>>> + Send
    where
        Self: Sync,
    {
        async {
            for (condition, output) in self.routes.iter() {
                if condition.evaluate(&event) {
                    return collector.send_named(output, event).await;
                }
            }
            collector.send_named(&self.fallback, event).await
        }
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use crate::components::collector::Collector;
    use crate::components::output::NamedOutput;
    use crate::event::metric::EventMetricValue;
    use crate::prelude::create_channel;
    use crate::transforms::condition;

    #[tokio::test]
    async fn should_route_events_properly() {
        use crate::transforms::Executable;

        let metrics_output = NamedOutput::named("metrics");
        let logs_output = NamedOutput::named("logs");
        let config = super::Config {
            routes: IndexMap::from_iter([
                (metrics_output.clone(), condition::Config::is_metric()),
                (logs_output.clone(), condition::Config::is_log()),
            ]),
            fallback: None,
        };
        let transform = config.build().unwrap();
        let (metric_tx, metric_rx) = create_channel(10);
        let (log_tx, log_rx) = create_channel(10);
        let collector = Collector::default()
            .with_output(metrics_output, metric_tx)
            .with_output(logs_output, log_tx);
        transform
            .handle(
                &collector,
                crate::event::log::EventLog::new("Hello World!")
                    .with_attribute("hostname", "fake-server")
                    .with_attribute("ddsource", "tiny-vector")
                    .into(),
            )
            .await
            .unwrap();
        transform
            .handle(
                &collector,
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
        assert_eq!(1, metric_rx.len());
        assert_eq!(1, log_rx.len());
    }

    #[tokio::test]
    async fn should_route_to_the_defined_fallback_route() {
        use crate::transforms::Executable;

        let metrics_output = NamedOutput::named("metrics");
        let fallback = NamedOutput::named("fallback");
        let config = super::Config {
            routes: IndexMap::from_iter([(metrics_output.clone(), condition::Config::is_metric())]),
            fallback: Some(fallback.clone()),
        };
        let transform = config.build().unwrap();
        let (fallback_tx, fallback_rx) = create_channel(10);
        let (metric_tx, metric_rx) = create_channel(10);
        let collector = Collector::default()
            .with_output(fallback, fallback_tx)
            .with_output(metrics_output, metric_tx);
        transform
            .handle(
                &collector,
                crate::event::log::EventLog::new("Hello World!")
                    .with_attribute("hostname", "fake-server")
                    .with_attribute("ddsource", "tiny-vector")
                    .into(),
            )
            .await
            .unwrap();
        transform
            .handle(
                &collector,
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
        assert_eq!(1, metric_rx.len());
        assert_eq!(1, fallback_rx.len());
    }

    #[tokio::test]
    async fn should_route_to_the_default_fallback_route() {
        use crate::transforms::Executable;

        let metrics_output = NamedOutput::named("metrics");
        let fallback = NamedOutput::named("dropped");
        let config = super::Config {
            routes: IndexMap::from_iter([(metrics_output.clone(), condition::Config::is_metric())]),
            fallback: None,
        };
        let transform = config.build().unwrap();
        let (fallback_tx, fallback_rx) = create_channel(10);
        let (metric_tx, metric_rx) = create_channel(10);
        let collector = Collector::default()
            .with_output(fallback, fallback_tx)
            .with_output(metrics_output, metric_tx);
        transform
            .handle(
                &collector,
                crate::event::log::EventLog::new("Hello World!")
                    .with_attribute("hostname", "fake-server")
                    .with_attribute("ddsource", "tiny-vector")
                    .into(),
            )
            .await
            .unwrap();
        transform
            .handle(
                &collector,
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
        assert_eq!(1, metric_rx.len());
        assert_eq!(1, fallback_rx.len());
    }

    #[test]
    fn should_break_the_build_to_have_conflicts() {
        let config = super::Config {
            routes: IndexMap::from_iter([(
                NamedOutput::named("dropped"),
                condition::Config::is_metric(),
            )]),
            fallback: None,
        };
        let err = config.build().unwrap_err();
        assert!(matches!(
            err,
            super::BuildError::FallbackRouteConflict { name: _ }
        ))
    }
}
