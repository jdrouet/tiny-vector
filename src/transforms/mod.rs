mod remove_fields;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error(transparent)]
    RemoveFields(#[from] self::remove_fields::BuildError),
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Config {
    RemoveFields(self::remove_fields::Config),
}

impl Config {
    pub fn build(
        self,
        incoming: crate::prelude::Sender,
    ) -> Result<(Transform, crate::prelude::Sender), BuildError> {
        Ok(match self {
            Self::RemoveFields(inner) => {
                let (tx, sender) = inner.build(incoming)?;
                (Transform::RemoveFields(tx), sender)
            }
        })
    }
}

pub enum Transform {
    RemoveFields(self::remove_fields::Transform),
}

impl Transform {
    pub async fn run(self, name: &str) -> tokio::task::JoinHandle<()> {
        match self {
            Self::RemoveFields(inner) => inner.run(name).await,
        }
    }
}
