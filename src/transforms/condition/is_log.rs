#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config;

impl Config {
    pub fn build(self) -> Condition {
        Condition
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Condition;

impl super::Evaluate for Condition {
    fn evaluate(&self, event: &crate::event::Event) -> bool {
        matches!(event, crate::event::Event::Log(_))
    }
}
