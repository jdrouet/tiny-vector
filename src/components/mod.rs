use std::cell::LazyCell;

use regex::Regex;

pub(crate) mod collector;
pub(crate) mod name;
pub(crate) mod output;

const NAME_REGEX: LazyCell<Regex> =
    LazyCell::new(|| Regex::new(r"^[a-zA-Z][a-zA-Z0-9\-_]*$").unwrap());

#[inline(always)]
fn validate_name(input: &str) -> bool {
    NAME_REGEX.is_match(input)
}

#[cfg(test)]
mod tests {
    #[test_case::test_case("foo"; "basic")]
    #[test_case::test_case("f"; "single character")]
    #[test_case::test_case("foo_bar"; "with lodash")]
    #[test_case::test_case("foo-bar"; "with dash")]
    #[test_case::test_case("foo-123-bar"; "with numbers")]
    fn should_validate(input: &str) {
        assert!(super::validate_name(input))
    }

    #[test_case::test_case(""; "empty")]
    #[test_case::test_case("   "; "only spaces")]
    #[test_case::test_case("foo bar"; "with space")]
    #[test_case::test_case("42_foo_bar"; "starting with number")]
    fn should_not_validate(input: &str) {
        assert!(!super::validate_name(input))
    }
}
