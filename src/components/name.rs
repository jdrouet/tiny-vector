use std::str::FromStr;

use super::validate_name;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComponentName(String);

impl TryFrom<String> for ComponentName {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if validate_name(&value) {
            Ok(Self(value))
        } else {
            Err("invalid output name format")
        }
    }
}

impl FromStr for ComponentName {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if validate_name(value) {
            Ok(Self(value.to_owned()))
        } else {
            Err("invalid component format")
        }
    }
}

#[cfg(test)]
impl ComponentName {
    pub fn new<T: Into<String>>(value: T) -> Self {
        Self(value.into())
    }
}

impl AsRef<str> for ComponentName {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl std::fmt::Display for ComponentName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ComponentName {
    pub fn into_string(self) -> String {
        self.0
    }
}

impl<'de> serde::de::Deserialize<'de> for ComponentName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_str(&value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::ComponentName;

    #[derive(Debug, serde::Deserialize)]
    struct Example {
        #[allow(dead_code)]
        values: HashMap<ComponentName, usize>,
    }

    #[test]
    fn should_deserialize() {
        let _result: Example = toml::from_str(
            r#"
values.foo-bar = 42
values.foo_bar = 42
values.f123 = 42
"#,
        )
        .unwrap();
    }

    #[test_case::test_case("values.\" \" = 32"; "empty")]
    #[test_case::test_case("values.\"foo$bar\" = 32"; "with special characters")]
    fn shouldnt_deserialize(template: &str) {
        let error = toml::from_str::<Example>(template).unwrap_err();
        assert!(error.to_string().contains("invalid format"));
    }
}
