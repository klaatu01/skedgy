use std::str::FromStr;

use chrono::{DateTime, Utc};
use cron::Schedule;
use serde::Deserialize;

pub(crate) fn serialize_datetime<S>(
    datetime: &DateTime<Utc>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&datetime.to_rfc3339())
}

pub(crate) fn deserialize_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&s)
        .map_err(serde::de::Error::custom)
        .map(|dt| dt.with_timezone(&Utc))
}

pub(crate) fn serialize_schedule<S>(schedule: &Schedule, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&schedule.to_string())
}

pub(crate) fn deserialize_schedule<'de, D>(deserializer: D) -> Result<Schedule, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Schedule::from_str(&s).map_err(serde::de::Error::custom)
}
