#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config {
    value: Vec<super::Config>,
}

impl super::prelude::Builder for Config {
    type Output = Condition;
    type Error = super::BuildError;

    fn build(self) -> Result<Condition, Self::Error> {
        Ok(Condition {
            value: self
                .value
                .into_iter()
                .map(|item| item.build())
                .collect::<Result<Vec<_>, super::BuildError>>()?,
        })
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
