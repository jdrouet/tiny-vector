#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum CheckConfig {
    Exists,
    Equals { value: String },
    EndsWith { value: String },
    Matches { regex: String },
    StartsWith { value: String },
}

impl Default for CheckConfig {
    fn default() -> Self {
        Self::Exists
    }
}

impl CheckConfig {
    fn build(self) -> Check {
        match self {
            Self::Exists => Check::Exists,
            Self::Equals { value } => Check::Equals { value },
            Self::EndsWith { value } => Check::EndsWith { value },
            Self::Matches { regex } => Check::Matches {
                regex: regex::Regex::new(regex.as_str()).unwrap(),
            },
            Self::StartsWith { value } => Check::StartsWith { value },
        }
    }
}

#[derive(Clone, Debug)]
pub enum Check {
    Exists,
    Equals { value: String },
    EndsWith { value: String },
    Matches { regex: regex::Regex },
    StartsWith { value: String },
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config {
    name: String,
    #[serde(default)]
    check: CheckConfig,
}

impl super::prelude::Builder for Config {
    type Output = Condition;
    type Error = super::BuildError;

    fn build(self) -> Result<Self::Output, Self::Error> {
        Ok(Condition {
            name: self.name,
            check: self.check.build(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Condition {
    name: String,
    check: Check,
}

impl super::prelude::Evaluate for Condition {
    fn evaluate(&self, event: &crate::event::Event) -> bool {
        event.as_event_metric().map_or(false, |m| match self.check {
            Check::Exists => m.header.tags.contains_key(self.name.as_str()),
            Check::Equals { ref value } => m
                .header
                .tags
                .get(self.name.as_str())
                .map_or(false, |v| value.eq(v.as_ref())),
            Check::EndsWith { ref value } => m
                .header
                .tags
                .get(self.name.as_str())
                .map_or(false, |v| v.ends_with(value)),
            Check::Matches { ref regex } => m
                .header
                .tags
                .get(self.name.as_str())
                .map_or(false, |v| regex.is_match(v.as_ref())),
            Check::StartsWith { ref value } => m
                .header
                .tags
                .get(self.name.as_str())
                .map_or(false, |v| v.starts_with(value)),
        })
    }
}
