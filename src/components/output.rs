use std::borrow::Cow;

use super::name::ComponentName;
use super::validate_name;
use crate::event::CowStr;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NamedOutput {
    Default,
    Named(CowStr),
}

#[cfg(test)]
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
        write!(f, "{}#{}", self.name, self.output)
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
        let result = AbstractComponentOutput::deserialize(deserializer)?;
        Ok(result.into())
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
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
        use crate::components::output::{AbstractComponentOutput, NamedOutput};

        #[test]
        fn with_simple_string() {
            let result: AbstractComponentOutput = serde_json::from_str(r#""foo""#).unwrap();
            assert!(matches!(result, AbstractComponentOutput::Default(_)));
        }

        #[test]
        fn with_just_component_name() {
            let result: AbstractComponentOutput =
                serde_json::from_str(r#"{"component": "foo"}"#).unwrap();
            assert!(matches!(
                result,
                AbstractComponentOutput::Named {
                    component: _,
                    output: NamedOutput::Default
                }
            ));
        }

        #[test]
        fn with_output_default() {
            let result: AbstractComponentOutput =
                serde_json::from_str(r#"{"component": "foo", "output": "default"}"#).unwrap();
            assert!(matches!(
                result,
                AbstractComponentOutput::Named {
                    component: _,
                    output: NamedOutput::Default
                }
            ));
        }

        #[test]
        fn with_output_named() {
            let result: AbstractComponentOutput =
                serde_json::from_str(r#"{"component": "foo", "output": "bar"}"#).unwrap();
            assert!(matches!(
                result,
                AbstractComponentOutput::Named {
                    component: _,
                    output: NamedOutput::Named(_)
                }
            ));
        }
    }
}
