#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config(Box<super::Config>);

impl super::prelude::Builder for Config {
    type Output = Condition;
    type Error = super::BuildError;

    fn build(self) -> Result<Self::Output, Self::Error> {
        Ok(Condition(Box::new(self.0.build()?)))
    }
}

#[derive(Clone, Debug)]
pub struct Condition(Box<super::Condition>);

impl super::prelude::Evaluate for Condition {
    fn evaluate(&self, event: &crate::event::Event) -> bool {
        !self.0.evaluate(event)
    }
}
