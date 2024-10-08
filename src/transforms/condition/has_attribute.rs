#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config {
    name: String,
}

impl super::prelude::Builder for Config {
    type Output = Condition;
    type Error = super::BuildError;

    fn build(self) -> Result<Self::Output, Self::Error> {
        Ok(Condition { name: self.name })
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Condition {
    name: String,
}

impl super::prelude::Evaluate for Condition {
    fn evaluate(&self, event: &crate::event::Event) -> bool {
        event
            .as_event_log()
            .map_or(false, |log| log.attributes.contains_key(self.name.as_str()))
    }
}
