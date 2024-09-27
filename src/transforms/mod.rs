use crate::components::collector::Collector;
use crate::components::name::ComponentName;
use crate::components::output::{ComponentWithOutputs, NamedOutput};
use crate::prelude::Receiver;

pub mod add_fields;
pub mod broadcast;
pub mod condition;
pub mod filter;
pub mod regex_parser;
pub mod remove_fields;
pub mod route;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error(transparent)]
    AddFields(#[from] self::add_fields::BuildError),
    #[error(transparent)]
    Filter(#[from] self::filter::BuildError),
    #[error(transparent)]
    RemoveFields(#[from] self::remove_fields::BuildError),
    #[error(transparent)]
    Route(#[from] self::route::BuildError),
}

#[derive(Clone, Debug, serde::Deserialize)]
#[enum_dispatch::enum_dispatch(ComponentWithOutputs)]
#[serde(rename_all = "snake_case", tag = "type")]
#[cfg_attr(test, derive(derive_more::From))]
pub enum Config {
    AddFields(self::add_fields::Config),
    Filter(self::filter::Config),
    RemoveFields(self::remove_fields::Config),
    Route(self::route::Config),
}

impl Config {
    pub fn build(self) -> Result<Transform, BuildError> {
        Ok(match self {
            Self::AddFields(inner) => Transform::AddFields(inner.build()?),
            Self::Filter(inner) => Transform::Filter(inner.build()?),
            Self::RemoveFields(inner) => Transform::RemoveFields(inner.build()?),
            Self::Route(inner) => Transform::Route(inner.build()?),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StartingError {}

pub enum Transform {
    AddFields(self::add_fields::Transform),
    Filter(self::filter::Transform),
    RemoveFields(self::remove_fields::Transform),
    Route(self::route::Transform),
}

impl Transform {
    pub async fn start(
        self,
        name: &ComponentName,
        receiver: Receiver,
        collector: Collector,
    ) -> Result<tokio::task::JoinHandle<()>, StartingError> {
        Ok(match self {
            Self::AddFields(inner) => inner.run(name, receiver, collector).await,
            Self::Filter(inner) => inner.run(name, receiver, collector).await,
            Self::RemoveFields(inner) => inner.run(name, receiver, collector).await,
            Self::Route(inner) => inner.run(name, receiver, collector).await,
        })
    }
}
