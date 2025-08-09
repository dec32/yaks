use std::time::Duration;

use rand::Rng;

pub struct PoliteDuration {
    base: Duration,
    jitter: u32,
}
impl PoliteDuration {
    pub const fn from_millis(base: u64, jitter_percentage: u32) -> Self {
        Self {
            base: Duration::from_millis(base),
            jitter: jitter_percentage,
        }
    }

    pub fn get(&self) -> Duration {
        let jitter = rand::rng().random_range(0..=self.jitter);
        self.base * (100 + jitter) / 100
    }
}
