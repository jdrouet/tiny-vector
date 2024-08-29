use super::name::ComponentName;
use super::validate_name;
use crate::event::CowStr;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NamedOutput {
    Default,
    Named(CowStr),
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
pub struct ComponentOutput {
    pub name: ComponentName,
    pub output: NamedOutput,
}

impl From<AbstractComponentOutput> for ComponentOutput {
    fn from(value: AbstractComponentOutput) -> Self {
        match value {
            AbstractComponentOutput::Default(name) => ComponentOutput {
                name,
                output: NamedOutput::Default,
            },
            AbstractComponentOutput::Named { component, output } => ComponentOutput {
                name: component,
                output,
            },
        }
    }
}

impl<'de> serde::de::Deserialize<'de> for ComponentOutput {
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
