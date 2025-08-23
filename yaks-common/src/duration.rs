use std::{ops::Range, time::Duration};

use rand::Rng;

pub struct RandomDuration(Range<u64>);
impl RandomDuration {
    pub const fn from_millis(range: Range<u64>) -> Self {
        Self(range)
    }

    pub fn get(&self) -> Duration {
        Duration::from_millis(rand::rng().random_range(self.0.clone()))
    }
}
