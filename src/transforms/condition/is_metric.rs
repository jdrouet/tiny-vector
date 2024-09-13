#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config;

impl super::prelude::Builder for Config {
    type Output = Condition;

    fn build(self) -> Condition {
        Condition
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Condition;

impl super::prelude::Evaluate for Condition {
    fn evaluate(&self, event: &crate::event::Event) -> bool {
        matches!(event, crate::event::Event::Metric(_))
    }
}
