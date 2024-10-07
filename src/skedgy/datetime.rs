use crate::{Skedgy, SkedgyContext, SkedgyHandler};
use std::borrow::Cow;

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

pub struct DateTime<'r, Ctx: SkedgyContext> {
    pub(crate) skedgy: Cow<'r, Skedgy<Ctx>>,
    pub(crate) datetime: chrono::DateTime<chrono::Utc>,
    pub(crate) schedule_builder: ScheduleBuilder,
}

impl<'r, Ctx: SkedgyContext> DateTime<'r, Ctx> {
    pub fn task<T: SkedgyHandler<Context = Ctx>>(self, handler: T) -> Handler<'r, Ctx, T> {
        let schedule_builder = self.schedule_builder.timestamp(self.datetime);
        Handler {
            skedgy: self.skedgy,
            schedule_builder,
            task: handler,
        }
    }
}
