use crate::{Skedgy, SkedgyContext, SkedgyHandler};
use std::borrow::Cow;

use super::{handler::Handler, schedule_builder::ScheduleBuilder};

pub struct Cron<'r, Ctx: SkedgyContext> {
    pub(crate) skedgy: Cow<'r, Skedgy<Ctx>>,
    pub(crate) cron: String,
    pub(crate) schedule_builder: ScheduleBuilder,
}

impl<'r, Ctx: SkedgyContext> Cron<'r, Ctx> {
    pub fn task<T: SkedgyHandler<Context = Ctx>>(self, handler: T) -> Handler<'r, Ctx, T> {
        let schedule_builder = self.schedule_builder.cron(&self.cron);
        Handler {
            skedgy: self.skedgy,
            task: handler,
            schedule_builder,
        }
    }
}
