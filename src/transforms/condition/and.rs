#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config {
    value: Vec<super::Config>,
}

impl super::prelude::Builder for Config {
    type Output = Condition;

    fn build(self) -> Condition {
        Condition {
            value: self.value.into_iter().map(|item| item.build()).collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Condition {
    value: Vec<super::Condition>,
}

impl super::prelude::Evaluate for Condition {
    fn evaluate(&self, event: &crate::event::Event) -> bool {
        self.value.iter().all(|item| item.evaluate(event))
    }
}
