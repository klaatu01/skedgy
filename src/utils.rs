#[cfg(feature = "serde")]
use serde::Deserialize;
#[cfg(feature = "serde")]
use std::str::FromStr;

#[cfg(feature = "serde")]
pub(crate) fn serialize_datetime<S>(
    datetime: &chrono::DateTime<chrono::Utc>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&datetime.to_rfc3339())
}

#[cfg(feature = "serde")]
pub(crate) fn deserialize_datetime<'de, D>(
    deserializer: D,
) -> Result<chrono::DateTime<chrono::Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    chrono::DateTime::parse_from_rfc3339(&s)
        .map_err(serde::de::Error::custom)
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

#[cfg(feature = "serde")]
pub(crate) fn serialize_schedule<S>(
    schedule: &cron::Schedule,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&schedule.to_string())
}

#[cfg(feature = "serde")]
pub(crate) fn deserialize_schedule<'de, D>(deserializer: D) -> Result<cron::Schedule, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    cron::Schedule::from_str(&s).map_err(serde::de::Error::custom)
}
