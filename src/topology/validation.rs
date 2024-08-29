#[derive(Debug, thiserror::Error)]
pub enum ValidationError {}

impl super::Config {
    pub fn validate(self) -> Result<Self, Vec<ValidationError>> {
        Ok(self)
    }
}
