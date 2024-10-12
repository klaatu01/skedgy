use crate::{task::Task, Skedgy};
use std::{borrow::Cow, sync::Arc};

use super::{handler::Handler, schedule_builder::ScheduleBuilder};

pub trait IntoDuration {
    fn into_duration(self) -> std::time::Duration;
}

impl IntoDuration for std::time::Duration {
    fn into_duration(self) -> std::time::Duration {
        self
    }
}

impl IntoDuration for chrono::Duration {
    fn into_duration(self) -> std::time::Duration {
        self.to_std().unwrap()
    }
}

pub struct Duration<'r> {
    pub(crate) skedgy: Cow<'r, Skedgy>,
    pub(crate) duration: std::time::Duration,
    pub(crate) schedule_builder: ScheduleBuilder,
}

impl<'r> Duration<'r> {
    pub fn task(self, task: impl Task + 'static) -> Handler<'r> {
        let schedule_builder = self.schedule_builder.duration(self.duration);
        Handler {
            skedgy: self.skedgy,
            task: Arc::new(task),
            schedule_builder,
        }
    }
}
