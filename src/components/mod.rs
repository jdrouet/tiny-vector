use std::cell::LazyCell;

use regex::Regex;

const NAME_REGEX: LazyCell<Regex> =
    LazyCell::new(|| Regex::new(r"^[a-zA-Z][a-zA-Z0-9\-_]*$").unwrap());

#[inline(always)]
fn validate_name(input: &str) -> bool {
    NAME_REGEX.is_match(input)
}

pub(crate) mod collector;
pub(crate) mod name;
pub(crate) mod output;
