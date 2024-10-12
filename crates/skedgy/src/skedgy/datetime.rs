use crate::{task::Task, Skedgy};
use std::{borrow::Cow, sync::Arc};

use super::{handler::Handler, schedule_builder::ScheduleBuilder};

pub trait IntoDateTime {
    fn into_datetime(self) -> chrono::DateTime<chrono::Utc>;
}

impl IntoDateTime for chrono::DateTime<chrono::Utc> {
    fn into_datetime(self) -> chrono::DateTime<chrono::Utc> {
        self
    }
}

impl IntoDateTime for std::time::SystemTime {
    fn into_datetime(self) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from(self)
    }
}

pub struct DateTime<'r> {
    pub(crate) skedgy: Cow<'r, Skedgy>,
    pub(crate) datetime: chrono::DateTime<chrono::Utc>,
    pub(crate) schedule_builder: ScheduleBuilder,
}

impl<'r> DateTime<'r> {
    pub fn task(self, handler: impl Task + 'static) -> Handler<'r> {
        let schedule_builder = self.schedule_builder.timestamp(self.datetime);
        Handler {
            skedgy: self.skedgy,
            schedule_builder,
            task: Arc::new(handler),
        }
    }
}
