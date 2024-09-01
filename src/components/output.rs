use std::borrow::Cow;
use std::str::FromStr;

use serde::Deserialize;

use super::name::ComponentName;
use super::validate_name;
use crate::event::CowStr;

pub trait ComponentWithOutputs {
    fn has_output(&self, output: &NamedOutput) -> bool {
        matches!(output, NamedOutput::Default)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NamedOutput {
    Default,
    Named(CowStr),
}

impl NamedOutput {
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Default)
    }
}

impl NamedOutput {
    pub fn named<N: Into<CowStr>>(name: N) -> Self {
        Self::Named(name.into())
    }
}

impl std::fmt::Display for NamedOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl TryFrom<String> for NamedOutput {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if validate_name(&value) {
            Ok(if value == "default" {
                NamedOutput::Default
            } else {
                NamedOutput::Named(CowStr::Owned(value))
            })
        } else {
            Err("invalid output name format")
        }
    }
}

impl FromStr for NamedOutput {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if validate_name(value) {
            Ok(if value == "default" {
                NamedOutput::Default
            } else {
                NamedOutput::Named(CowStr::Owned(value.to_owned()))
            })
        } else {
            Err("invalid output name format")
        }
    }
}

impl<'de> serde::de::Deserialize<'de> for NamedOutput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        if validate_name(&value) {
            Ok(if value == "default" {
                NamedOutput::Default
            } else {
                NamedOutput::Named(CowStr::Owned(value))
            })
        } else {
            Err(serde::de::Error::custom("invalid format"))
        }
    }
}

impl Default for NamedOutput {
    fn default() -> Self {
        Self::Default
    }
}

impl AsRef<str> for NamedOutput {
    fn as_ref(&self) -> &str {
        match self {
            Self::Default => "default",
            Self::Named(inner) => inner.as_ref(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ComponentOutput<'a> {
    pub name: Cow<'a, ComponentName>,
    pub output: Cow<'a, NamedOutput>,
}

impl<'a> std::fmt::Display for ComponentOutput<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.output.as_ref().is_default() {
            self.name.as_ref().fmt(f)
        } else {
            write!(f, "{}#{}", self.name, self.output)
        }
    }
}

impl FromStr for ComponentOutput<'static> {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if let Some((component, output)) = s.split_once('#') {
            Self {
                name: Cow::Owned(ComponentName::from_str(component)?),
                output: Cow::Owned(NamedOutput::from_str(output)?),
            }
        } else {
            Self {
                name: Cow::Owned(ComponentName::from_str(s)?),
                output: Cow::Owned(NamedOutput::Default),
            }
        })
    }
}

impl TryFrom<String> for ComponentOutput<'static> {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(if let Some((component, output)) = value.split_once('#') {
            Self {
                name: Cow::Owned(ComponentName::from_str(component)?),
                output: Cow::Owned(NamedOutput::from_str(output)?),
            }
        } else {
            Self {
                name: Cow::Owned(ComponentName::try_from(value)?),
                output: Cow::Owned(NamedOutput::Default),
            }
        })
    }
}

impl<'a> ComponentOutput<'a> {
    pub fn to_owned(&self) -> ComponentOutput<'static> {
        ComponentOutput::<'static> {
            name: Cow::Owned(self.to_owned_name()),
            output: Cow::Owned(self.to_owned_output()),
        }
    }

    pub fn to_borrowed<'b>(&'a self) -> ComponentOutput<'b>
    where
        'a: 'b,
    {
        ComponentOutput {
            name: Cow::Borrowed(self.name.as_ref()),
            output: Cow::Borrowed(self.output.as_ref()),
        }
    }

    pub fn to_owned_name(&self) -> ComponentName {
        match self.name {
            Cow::Owned(ref inner) => inner.clone(),
            Cow::Borrowed(inner) => inner.clone(),
        }
    }

    pub fn to_owned_output(&self) -> NamedOutput {
        match self.output {
            Cow::Owned(ref inner) => inner.clone(),
            Cow::Borrowed(inner) => inner.clone(),
        }
    }
}

impl From<AbstractComponentOutput> for ComponentOutput<'static> {
    fn from(value: AbstractComponentOutput) -> Self {
        match value {
            AbstractComponentOutput::Default(name) => ComponentOutput {
                name: Cow::Owned(name),
                output: Cow::Owned(NamedOutput::Default),
            },
            AbstractComponentOutput::Named { component, output } => ComponentOutput {
                name: Cow::Owned(component),
                output: Cow::Owned(output),
            },
        }
    }
}

impl<'de> serde::de::Deserialize<'de> for ComponentOutput<'static> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ComponentOutputVisitor)
    }
}

struct ComponentOutputVisitor;

impl<'de> serde::de::Visitor<'de> for ComponentOutputVisitor {
    type Value = ComponentOutput<'static>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string or a map")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        ComponentOutput::from_str(value).map_err(serde::de::Error::custom)
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        AbstractComponentOutput::deserialize(serde::de::value::MapAccessDeserializer::new(map))
            .map(ComponentOutput::from)
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
#[serde(untagged)]
enum AbstractComponentOutput {
    Default(ComponentName),
    Named {
        component: ComponentName,
        #[serde(default)]
        output: NamedOutput,
    },
}

#[cfg(test)]
mod tests {
    mod should_deserialize_abstract_component_output {
        use std::borrow::Cow;

        use crate::components::name::ComponentName;
        use crate::components::output::{ComponentOutput, NamedOutput};

        #[test]
        fn with_simple_string() {
            let result: ComponentOutput<'static> = serde_json::from_str(r#""foo""#).unwrap();
            assert_eq!(
                result,
                ComponentOutput {
                    name: Cow::Owned(ComponentName::new("foo")),
                    output: Cow::Owned(NamedOutput::Default),
                }
            );
        }

        #[test]
        fn with_hash() {
            let result: ComponentOutput<'static> = serde_json::from_str(r#""foo#bar""#).unwrap();
            assert_eq!(
                result,
                ComponentOutput {
                    name: Cow::Owned(ComponentName::new("foo")),
                    output: Cow::Owned(NamedOutput::named("bar")),
                }
            );
        }

        #[test]
        fn with_just_component_name() {
            let result: ComponentOutput<'static> =
                serde_json::from_str(r#"{"component": "foo"}"#).unwrap();
            assert_eq!(
                result,
                ComponentOutput {
                    name: Cow::Owned(ComponentName::new("foo")),
                    output: Cow::Owned(NamedOutput::Default),
                }
            );
        }

        #[test]
        fn with_output_default() {
            let result: ComponentOutput<'static> =
                serde_json::from_str(r#"{"component": "foo", "output": "default"}"#).unwrap();
            assert_eq!(
                result,
                ComponentOutput {
                    name: Cow::Owned(ComponentName::new("foo")),
                    output: Cow::Owned(NamedOutput::Default),
                }
            );
        }

        #[test]
        fn with_output_named() {
            let result: ComponentOutput<'static> =
                serde_json::from_str(r#"{"component": "foo", "output": "bar"}"#).unwrap();
            assert_eq!(
                result,
                ComponentOutput {
                    name: Cow::Owned(ComponentName::new("foo")),
                    output: Cow::Owned(NamedOutput::named("bar")),
                }
            );
        }
    }
}
