use crate::{task::Task, Skedgy};
use std::{borrow::Cow, sync::Arc};

use super::{handler::Handler, schedule_builder::ScheduleBuilder};

pub struct Cron<'r> {
    pub(crate) skedgy: Cow<'r, Skedgy>,
    pub(crate) cron: String,
    pub(crate) schedule_builder: ScheduleBuilder,
}

impl<'r> Cron<'r> {
    pub fn task(self, task: impl Task + 'static) -> Handler<'r> {
        let schedule_builder = self.schedule_builder.cron(&self.cron);
        Handler {
            skedgy: self.skedgy,
            task: Arc::new(task),
            schedule_builder,
        }
    }
}
