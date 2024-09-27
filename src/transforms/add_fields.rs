use indexmap::IndexMap;

use crate::components::output::ComponentWithOutputs;
use crate::event::Event;
use crate::prelude::StringOrEnv;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {}

#[derive(Clone, Debug, serde::Deserialize)]
#[cfg_attr(test, derive(Default))]
pub struct Config {
    fields: IndexMap<String, StringOrEnv>,
}

impl ComponentWithOutputs for Config {}

impl Config {
    pub fn build(self) -> Result<Transform, BuildError> {
        Ok(Transform {
            fields: self
                .fields
                .into_iter()
                .filter_map(|(name, value)| value.into_string().map(|v| (name, v)))
                .collect(),
        })
    }
}

pub struct Transform {
    fields: IndexMap<String, String>,
}

impl Transform {
    pub(crate) fn flavor(&self) -> &'static str {
        "add_fields"
    }
}

impl super::Executable for Transform {
    fn transform(&self, event: Event) -> Event {
        match event {
            Event::Log(mut inner) => {
                for (name, value) in self.fields.iter() {
                    inner.add_attribute(name.clone(), value.clone());
                }
                Event::Log(inner)
            }
            Event::Metric(mut inner) => {
                for (name, value) in self.fields.iter() {
                    inner.add_tag(name.clone(), value.clone());
                }
                Event::Metric(inner)
            }
        }
    }
}
