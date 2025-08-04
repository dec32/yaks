use std::str::FromStr;

use anyhow::anyhow;

pub struct Range(u64, u64);

impl Range {
    pub fn contains(&self, value: u64) -> bool {
        self.0 <= value && value <= self.1
    }
}

impl Default for Range {
    fn default() -> Self {
        Self(0, u64::MAX)
    }
}

impl FromStr for Range {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let split = s
            .split_once("~")
            .ok_or(anyhow!("Illegal range literal {s}."))?;
        let start = if split.0.is_empty() {
            0
        } else {
            split.0.parse()?
        };
        let end = if split.1.is_empty() {
            u64::MAX
        } else {
            split.1.parse()?
        };
        Ok(Self(start, end))
    }
}
