use std::{collections::BTreeMap, error::Error, str::FromStr, time::Duration};

use chrono::{DateTime, Utc};
use cron::Schedule;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};

/// Configuration for the `Skedgy` scheduler.
#[derive(Debug, Clone)]
pub struct SkedgyConfig {
    pub tick_interval: Duration,
}
