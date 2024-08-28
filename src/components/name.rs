use super::validate_name;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComponentName(String);

impl From<String> for ComponentName {
    fn from(value: String) -> Self {
        Self(value)
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
        if validate_name(&value) {
            Ok(Self(value))
        } else {
            Err(serde::de::Error::custom("invalid format"))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::ComponentName;

    #[test_case::test_case("foo"; "basic")]
    #[test_case::test_case("f"; "single character")]
    #[test_case::test_case("foo_bar"; "with lodash")]
    #[test_case::test_case("foo-bar"; "with dash")]
    #[test_case::test_case("foo-123-bar"; "with numbers")]
    fn should_validate(input: &str) {
        assert!(super::validate_component_name(input))
    }

    #[test_case::test_case(""; "empty")]
    #[test_case::test_case("   "; "only spaces")]
    #[test_case::test_case("foo bar"; "with space")]
    #[test_case::test_case("42_foo_bar"; "starting with number")]
    fn should_not_validate(input: &str) {
        assert!(!super::validate_component_name(input))
    }

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
