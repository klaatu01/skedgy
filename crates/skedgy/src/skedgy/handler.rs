use std::{borrow::Cow, future::IntoFuture, sync::Arc};

use futures::future::BoxFuture;

use crate::task::Task;

use super::{schedule_builder::ScheduleBuilder, Skedgy};

pub struct Handler<'r> {
    pub(crate) skedgy: Cow<'r, Skedgy>,
    pub(crate) schedule_builder: ScheduleBuilder,
    pub(crate) task: Arc<dyn Task>,
}

impl<'r> IntoFuture for Handler<'r> {
    type Output = Result<(), crate::error::SkedgyError>;
    type IntoFuture = BoxFuture<'r, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        let task = self.schedule_builder.task(self.task).build();
        Box::pin(async move {
            match task {
                Ok(task) => self.skedgy.schedule(task).await,
                Err(e) => Err(e),
            }
        })
    }
}
