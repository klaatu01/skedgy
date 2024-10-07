use crate::{Skedgy, SkedgyContext};
use std::borrow::Cow;

use super::{
    cron::Cron,
    datetime::{DateTime, IntoDateTime},
    duration::{Duration, IntoDuration},
    schedule_builder::ScheduleBuilder,
};

pub struct Named<'r, Ctx: SkedgyContext> {
    pub(crate) skedgy: Cow<'r, Skedgy<Ctx>>,
    pub(crate) id: String,
    pub(crate) schedule_builder: ScheduleBuilder,
}

impl<'r, Ctx: SkedgyContext> Named<'r, Ctx> {
    pub fn datetime(self, datetime: impl IntoDateTime) -> DateTime<'r, Ctx> {
        let schedule_builder = self.schedule_builder.id(&self.id);
        DateTime {
            skedgy: self.skedgy,
            datetime: datetime.into_datetime(),
            schedule_builder,
        }
    }

    pub fn duration(self, duration: impl IntoDuration) -> Duration<'r, Ctx> {
        let schedule_builder = self.schedule_builder.id(&self.id);
        Duration {
            skedgy: self.skedgy,
            duration: duration.into_duration(),
            schedule_builder,
        }
    }

    pub fn cron(self, cron: &str) -> Cron<'r, Ctx> {
        let schedule_builder = self.schedule_builder.id(&self.id);
        Cron {
            skedgy: self.skedgy,
            cron: cron.to_string(),
            schedule_builder,
        }
    }
}
