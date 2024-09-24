use std::time::Duration;

/// Configuration for the `Skedgy` scheduler.
#[derive(Debug, Clone)]
pub struct SkedgyConfig {
    pub look_ahead_duration: Duration,
}

impl Default for SkedgyConfig {
    fn default() -> Self {
        Self {
            look_ahead_duration: Duration::from_millis(10),
        }
    }
}
