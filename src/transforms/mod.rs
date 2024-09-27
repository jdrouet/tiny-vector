use tokio::sync::mpsc::error::SendError;

use crate::components::collector::Collector;
use crate::components::name::ComponentName;
use crate::components::output::{ComponentWithOutputs, NamedOutput};
use crate::event::Event;
use crate::prelude::Receiver;

pub mod add_fields;
pub mod broadcast;
pub mod condition;
pub mod filter;
pub mod regex_parser;
pub mod remove_fields;
pub mod route;

const COMPONENT_KIND: &str = "transform";

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error(transparent)]
    AddFields(#[from] self::add_fields::BuildError),
    #[error(transparent)]
    Broadcast(#[from] self::broadcast::BuildError),
    #[error(transparent)]
    Filter(#[from] self::filter::BuildError),
    #[error(transparent)]
    RegexParser(#[from] self::regex_parser::BuildError),
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
    Broadcast(self::broadcast::Config),
    Filter(self::filter::Config),
    RegexParser(self::regex_parser::Config),
    RemoveFields(self::remove_fields::Config),
    Route(self::route::Config),
}

impl Config {
    pub fn build(self) -> Result<Transform, BuildError> {
        Ok(match self {
            Self::AddFields(inner) => Transform::AddFields(inner.build()?),
            Self::Broadcast(inner) => Transform::Broadcast(inner.build()?),
            Self::Filter(inner) => Transform::Filter(inner.build()?),
            Self::RegexParser(inner) => Transform::RegexParser(inner.build()?),
            Self::RemoveFields(inner) => Transform::RemoveFields(inner.build()?),
            Self::Route(inner) => Transform::Route(inner.build()?),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StartingError {}

pub enum Transform {
    AddFields(self::add_fields::Transform),
    Broadcast(self::broadcast::Transform),
    Filter(self::filter::Transform),
    RegexParser(self::regex_parser::Transform),
    RemoveFields(self::remove_fields::Transform),
    Route(self::route::Transform),
}

impl Transform {
    fn flavor(&self) -> &'static str {
        match self {
            Self::AddFields(inner) => inner.flavor(),
            Self::Broadcast(inner) => inner.flavor(),
            Self::Filter(inner) => inner.flavor(),
            Self::RegexParser(inner) => inner.flavor(),
            Self::RemoveFields(inner) => inner.flavor(),
            Self::Route(inner) => inner.flavor(),
        }
    }

    pub async fn start(
        self,
        name: &ComponentName,
        receiver: Receiver,
        collector: Collector,
    ) -> Result<tokio::task::JoinHandle<()>, StartingError> {
        let span = tracing::info_span!(
            "component",
            name = name.as_ref(),
            kind = COMPONENT_KIND,
            flavor = self.flavor(),
        );
        Ok(match self {
            Self::AddFields(inner) => run(inner, span, receiver, collector).await?,
            Self::Broadcast(inner) => run(inner, span, receiver, collector).await?,
            Self::Filter(inner) => run(inner, span, receiver, collector).await?,
            Self::RegexParser(inner) => run(inner, span, receiver, collector).await?,
            Self::RemoveFields(inner) => run(inner, span, receiver, collector).await?,
            Self::Route(inner) => run(inner, span, receiver, collector).await?,
        })
    }
}

trait Executable: Sized {
    #[inline]
    fn transform(&self, event: Event) -> Event {
        event
    }

    fn handle(
        &self,
        collector: &Collector,
        event: Event,
    ) -> impl std::future::Future<Output = Result<(), SendError<Event>>> + Send
    where
        Self: Sync,
    {
        async { collector.send_default(self.transform(event)).await }
    }

    fn execute(
        self,
        mut receiver: Receiver,
        collector: Collector,
    ) -> impl std::future::Future<Output = ()> + Send
    where
        Self: Send + Sync,
    {
        async move {
            tracing::info!("starting");
            while let Some(event) = receiver.recv().await {
                if let Err(err) = self.handle(&collector, event).await {
                    tracing::error!("unable to route event: {err:?}");
                    break;
                }
            }
            tracing::info!("stopping");
        }
    }
}

async fn run<E: Executable + Send + Sized + Sync + 'static>(
    element: E,
    span: tracing::Span,
    receiver: Receiver,
    collector: Collector,
) -> Result<tokio::task::JoinHandle<()>, StartingError> {
    use tracing::Instrument;

    Ok(tokio::spawn(async move {
        element.execute(receiver, collector).instrument(span).await
    }))
}
