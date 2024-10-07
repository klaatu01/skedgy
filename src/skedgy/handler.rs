use std::{borrow::Cow, future::IntoFuture};

use futures::future::BoxFuture;

use crate::{SkedgyContext, SkedgyHandler};

use super::{schedule_builder::ScheduleBuilder, Skedgy};

pub struct Handler<'r, Ctx: SkedgyContext, T: SkedgyHandler<Context = Ctx>> {
    pub(crate) skedgy: Cow<'r, Skedgy<Ctx>>,
    pub(crate) schedule_builder: ScheduleBuilder,
    pub(crate) task: T,
}

impl<'r, Ctx: SkedgyContext, T: SkedgyHandler<Context = Ctx>> IntoFuture for Handler<'r, Ctx, T> {
    type Output = Result<(), crate::error::SkedgyError>;
    type IntoFuture = BoxFuture<'r, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let task = self.schedule_builder.task(self.task);
            match task {
                Ok(task) => self.skedgy.schedule(task).await,
                Err(e) => Err(e),
            }
        })
    }
}
