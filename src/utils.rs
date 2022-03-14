   
use regex;
use anyhow::Result;

pub fn build_regex(pattern: &str) -> Result<regex::Regex> {
    let pattern = format!("^{}.*", pattern);
    let re = regex::RegexBuilder::new(&pattern).build()?;
    Ok(re)
}