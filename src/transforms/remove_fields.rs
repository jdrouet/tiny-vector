use indexmap::IndexSet;

use crate::components::output::ComponentWithOutputs;
use crate::event::Event;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config {
    fields: IndexSet<String>,
}

impl ComponentWithOutputs for Config {}

impl Config {
    pub fn build(self) -> Result<Transform, BuildError> {
        Ok(Transform {
            fields: self.fields,
        })
    }
}

pub struct Transform {
    fields: IndexSet<String>,
}

impl Transform {
    pub(crate) fn flavor(&self) -> &'static str {
        "remove_fields"
    }
}

impl super::Executable for Transform {
    fn transform(&self, event: Event) -> Event {
        match event {
            Event::Log(mut inner) => {
                inner
                    .attributes
                    .retain(|key, _| !self.fields.contains(key.as_ref()));
                Event::Log(inner)
            }
            Event::Metric(mut inner) => {
                inner
                    .header
                    .tags
                    .retain(|key, _| !self.fields.contains(key.as_ref()));
                Event::Metric(inner)
            }
        }
    }
}
