use crate::components::output::ComponentWithOutputs;
use crate::event::log::EventLog;
use crate::event::Event;

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
    pub(crate) fn flavor(&self) -> &'static str {
        "regex_parser"
    }

    fn handle_log(&self, event_log: EventLog) -> EventLog {
        let EventLog {
            mut attributes,
            message,
        } = event_log;
        let mut new_message = None::<String>;
        if let Some(capture) = self.pattern.captures(&message) {
            for name in self.pattern.capture_names().flatten() {
                match capture.name(name) {
                    Some(value) if name == "message" => {
                        new_message = Some(value.as_str().to_owned());
                    }
                    Some(value) => {
                        attributes.insert(name.to_owned().into(), value.as_str().to_owned().into());
                    }
                    None => {}
                }
            }
            EventLog {
                attributes,
                message: new_message.unwrap_or(message),
            }
        } else {
            EventLog {
                attributes,
                message,
            }
        }
    }
}

impl super::Executable for Transform {
    fn transform(&self, event: Event) -> Event {
        match event {
            Event::Log(inner) => Event::Log(self.handle_log(inner)),
            Event::Metric(inner) => Event::Metric(inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::event::metric::EventMetricValue;

    #[tokio::test]
    async fn should_extract_from_logs() {
        use crate::transforms::Executable;

        let config = super::Config {
            pattern: String::from(
                r"^service=(?<service>[a-z]+)\s+status=(?<status>[a-z]+)\s+(?<message>.*)$",
            ),
        };
        let transform = config.build().unwrap();

        let event = transform.transform(
            crate::event::metric::EventMetric::new(
                crate::helper::now(),
                "foo",
                "bar",
                EventMetricValue::Gauge(42.0),
            )
            .with_tag("hostname", "fake-server")
            .into(),
        );
        let event = event.into_event_metric().unwrap();
        assert_eq!(event.value, EventMetricValue::Gauge(42.0));

        let event = transform.transform(
            crate::event::log::EventLog::new("service=something status=ok hello world").into(),
        );
        let event = event.into_event_log().unwrap();
        assert_eq!(event.message, "hello world");
        assert_eq!(
            event
                .attributes
                .get("service")
                .and_then(|v| v.as_text())
                .unwrap(),
            "something"
        );
        assert_eq!(
            event
                .attributes
                .get("status")
                .and_then(|v| v.as_text())
                .unwrap(),
            "ok"
        );

        let event = transform
            .transform(crate::event::log::EventLog::new("whatever status=ok hello world").into());
        let event = event.into_event_log().unwrap();
        assert_eq!(event.message, "whatever status=ok hello world");
    }
}
