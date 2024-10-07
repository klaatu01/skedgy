use std::str::FromStr;

use nanoid::nanoid;

use crate::{task::SkedgyTask, SkedgyError};

pub enum ScheduleType {
    Duration(std::time::Duration),
    Cron(String),
    Timestamp(chrono::DateTime<chrono::Utc>),
}

pub(crate) struct ScheduleBuilder {
    pub(crate) id: Option<String>,
    pub(crate) schedule_type: Option<ScheduleType>,
}

impl ScheduleBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            schedule_type: None,
        }
    }

    pub fn id(mut self, id: &str) -> Self {
        self.id = Some(id.to_string());
        self
    }

    pub fn duration(mut self, duration: std::time::Duration) -> Self {
        self.schedule_type = Some(ScheduleType::Duration(duration));
        self
    }

    pub fn cron(mut self, cron: &str) -> Self {
        self.schedule_type = Some(ScheduleType::Cron(cron.to_string()));
        self
    }

    pub fn timestamp(mut self, timestamp: chrono::DateTime<chrono::Utc>) -> Self {
        self.schedule_type = Some(ScheduleType::Timestamp(timestamp));
        self
    }

    pub fn task<Ctx, T>(self, handler: T) -> Result<SkedgyTask<Ctx, T>, SkedgyError>
    where
        Ctx: crate::SkedgyContext,
        T: crate::SkedgyHandler<Context = Ctx>,
    {
        let id = self.id.unwrap_or_else(|| nanoid!(10));
        let kind = self.schedule_type.ok_or(SkedgyError::NoSchedule)?;
        let kind = match kind {
            ScheduleType::Duration(duration) => crate::task::TaskKind::In(duration),
            ScheduleType::Cron(cron) => {
                let schedule =
                    cron::Schedule::from_str(&cron).map_err(|_| SkedgyError::InvalidCron)?;
                crate::task::TaskKind::Cron(schedule)
            }
            ScheduleType::Timestamp(timestamp) => crate::task::TaskKind::At(timestamp),
        };
        Ok(SkedgyTask { id, kind, handler })
    }
}
