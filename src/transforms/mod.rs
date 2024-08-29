use std::collections::HashSet;

use crate::components::collector::Collector;
use crate::components::name::ComponentName;
use crate::components::output::NamedOutput;
use crate::prelude::Receiver;

mod add_fields;
mod remove_fields;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error(transparent)]
    AddFields(#[from] self::add_fields::BuildError),
    #[error(transparent)]
    RemoveFields(#[from] self::remove_fields::BuildError),
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Config {
    AddFields(self::add_fields::Config),
    RemoveFields(self::remove_fields::Config),
}

impl Config {
    pub fn outputs(&self) -> HashSet<NamedOutput> {
        HashSet::from_iter([NamedOutput::Default])
    }

    pub fn build(self) -> Result<Transform, BuildError> {
        Ok(match self {
            Self::AddFields(inner) => Transform::AddFields(inner.build()?),
            Self::RemoveFields(inner) => Transform::RemoveFields(inner.build()?),
        })
    }
}

pub enum Transform {
    AddFields(self::add_fields::Transform),
    RemoveFields(self::remove_fields::Transform),
}

impl Transform {
    pub async fn run(
        self,
        name: &ComponentName,
        receiver: Receiver,
        collector: Collector,
    ) -> tokio::task::JoinHandle<()> {
        match self {
            Self::AddFields(inner) => inner.run(name, receiver, collector).await,
            Self::RemoveFields(inner) => inner.run(name, receiver, collector).await,
        }
    }
}
