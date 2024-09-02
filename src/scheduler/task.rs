use std::{collections::BTreeMap, error::Error, str::FromStr, time::Duration};

use chrono::{DateTime, Utc};
use cron::Schedule;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};

use crate::{
    error::SkedgyError,
    handler::SkedgyHandler,
    utils::{deserialize_datetime, deserialize_schedule, serialize_datetime, serialize_schedule},
};

#[derive(Clone, Serialize, Deserialize)]
pub enum TaskKind {
    #[serde(
        serialize_with = "serialize_datetime",
        deserialize_with = "deserialize_datetime"
    )]
    At(DateTime<Utc>),

    In(Duration),

    #[serde(
        serialize_with = "serialize_schedule",
        deserialize_with = "deserialize_schedule"
    )]
    Cron(Schedule),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SkedgyTask<T: SkedgyHandler> {
    pub(crate) id: String,
    pub(crate) kind: TaskKind,
    pub(crate) handler: T,
}

impl<T: SkedgyHandler> SkedgyTask<T> {
    pub fn named(id: &str) -> SkedgyTaskBuilder<T> {
        SkedgyTaskBuilder::named(id)
    }

    pub fn anonymous() -> SkedgyTaskBuilder<T> {
        SkedgyTaskBuilder::new()
    }
}

pub struct SkedgyTaskBuilder<T: SkedgyHandler> {
    kind: Option<TaskKind>,
    handler: Option<T>,
    id: Option<String>,
}

impl<T: SkedgyHandler> SkedgyTaskBuilder<T> {
    pub fn named(id: &str) -> Self {
        Self {
            kind: None,
            handler: None,
            id: Some(id.to_string()),
        }
    }

    pub fn new() -> Self {
        Self {
            kind: None,
            handler: None,
            id: nanoid!(10).into(),
        }
    }

    pub fn at(&mut self, datetime: DateTime<Utc>) -> &mut Self {
        self.kind = Some(TaskKind::At(datetime));
        self
    }

    pub fn r#in(&mut self, duration: Duration) -> &mut Self {
        let datetime = Utc::now() + duration;
        self.kind = Some(TaskKind::At(datetime));
        self
    }

    pub fn cron(&mut self, pattern: &str) -> Result<&mut Self, SkedgyError> {
        let schedule = cron::Schedule::from_str(pattern).map_err(|_| SkedgyError::InvalidCron)?;
        self.kind = Some(TaskKind::Cron(schedule));
        Ok(self)
    }

    pub fn handler(&mut self, handler: T) -> &mut Self {
        self.handler = Some(handler);
        self
    }

    pub fn build(&self) -> Result<SkedgyTask<T>, SkedgyError> {
        let kind = self.kind.clone().ok_or(SkedgyError::InvalidCron)?;
        let handler = self.handler.clone().ok_or(SkedgyError::InvalidCron)?;
        let id = self.id.clone().unwrap_or_else(|| nanoid!(10));
        Ok(SkedgyTask { id, kind, handler })
    }
}
