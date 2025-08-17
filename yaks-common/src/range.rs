use std::{
    cmp::Ordering,
    fmt::{self, Display},
    str::FromStr,
};

use anyhow::{Result, bail};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Range {
    pub start: u64,
    pub end: u64,
}

impl Range {
    pub fn contains(&self, val: &u64) -> bool {
        self.start <= *val && *val <= self.end
    }
}

impl Default for Range {
    fn default() -> Self {
        Self {
            start: 0,
            end: u64::MAX,
        }
    }
}

impl PartialEq<u64> for Range {
    fn eq(&self, other: &u64) -> bool {
        self.contains(other)
    }
}

impl FromStr for Range {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split("..").collect();
        let (start, end) = match parts.as_slice() {
            [s_str, e_str] => {
                let start = if s_str.is_empty() { 0 } else { s_str.parse()? };
                let end = if e_str.is_empty() {
                    u64::MAX
                } else {
                    e_str.parse::<u64>()? - 1
                };
                (start, end)
            }
            _ => bail!("Invalid range format"),
        };
        Ok(Range { start, end })
    }
}

impl Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start == 0 && self.end == u64::MAX {
            write!(f, "~")
        } else if self.start == 0 {
            write!(f, "~{}", self.end)
        } else if self.end == u64::MAX {
            write!(f, "{}~", self.start)
        } else {
            write!(f, "{}~{}", self.start, self.end)
        }
    }
}

impl PartialOrd<u64> for Range {
    fn partial_cmp(&self, other: &u64) -> Option<Ordering> {
        if self.contains(other) {
            Some(Ordering::Equal)
        } else if other < &self.start {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Less)
        }
    }
}

impl PartialEq<Range> for u64 {
    fn eq(&self, other: &Range) -> bool {
        other.contains(self)
    }
}

impl PartialOrd<Range> for u64 {
    fn partial_cmp(&self, other: &Range) -> Option<Ordering> {
        if other.contains(self) {
            Some(Ordering::Equal)
        } else if self < &other.start {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Greater)
        }
    }
}
