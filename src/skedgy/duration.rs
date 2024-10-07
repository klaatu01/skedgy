use crate::{Skedgy, SkedgyContext, SkedgyHandler};
use std::borrow::Cow;

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

pub struct Duration<'r, Ctx: SkedgyContext> {
    pub(crate) skedgy: Cow<'r, Skedgy<Ctx>>,
    pub(crate) duration: std::time::Duration,
    pub(crate) schedule_builder: ScheduleBuilder,
}

impl<'r, Ctx: SkedgyContext> Duration<'r, Ctx> {
    pub fn task<T: SkedgyHandler<Context = Ctx>>(self, handler: T) -> Handler<'r, Ctx, T> {
        let schedule_builder = self.schedule_builder.duration(self.duration);
        Handler {
            skedgy: self.skedgy,
            task: handler,
            schedule_builder,
        }
    }
}
