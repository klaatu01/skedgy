use std::{str::FromStr, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use cron::Schedule;
use nanoid::nanoid;

use crate::{
    error::SkedgyError,
    handler::{DynHandler, SkedgyHandler},
    Metadata, SkedgyContext,
};

#[derive(Clone)]
pub enum TaskKind {
    At(DateTime<Utc>),
    In(Duration),
    Cron(Schedule),
}

#[derive(Clone)]
pub struct DynSkedgyTask<Ctx: SkedgyContext> {
    pub(crate) id: String,
    pub(crate) kind: TaskKind,
    pub(crate) handler: Box<dyn DynHandler<Ctx>>,
}

impl<Ctx: SkedgyContext, T: SkedgyHandler<Context = Ctx>> From<SkedgyTask<Ctx, T>>
    for DynSkedgyTask<Ctx>
{
    fn from(task: SkedgyTask<Ctx, T>) -> Self {
        Self {
            id: task.id,
            kind: task.kind,
            handler: Box::new(task.handler),
        }
    }
}

impl<Ctx: SkedgyContext> DynSkedgyTask<Ctx> {
    pub async fn execute(self, context: &Ctx, metadata: Metadata) {
        self.handler.clone().handle_dyn(context, metadata).await;
    }
}

#[derive(Clone)]
pub struct SkedgyTask<Ctx: SkedgyContext, T: SkedgyHandler<Context = Ctx>> {
    pub(crate) id: String,
    pub(crate) kind: TaskKind,
    pub(crate) handler: T,
}

impl<Ctx: SkedgyContext, T: SkedgyHandler<Context = Ctx>> SkedgyTask<Ctx, T> {
    pub fn named(id: &str) -> SkedgyTaskBuilder<Ctx, T> {
        SkedgyTaskBuilder::named(id)
    }

    pub fn anonymous() -> SkedgyTaskBuilder<Ctx, T> {
        SkedgyTaskBuilder::new()
    }

    pub async fn execute(&self, context: &Ctx, metadata: Metadata) {
        self.handler.handle(context, metadata).await;
    }
}

pub struct SkedgyTaskBuilder<Ctx: SkedgyContext, T: SkedgyHandler<Context = Ctx>> {
    kind: Option<TaskKind>,
    handler: Option<T>,
    id: Option<String>,
}

impl<Ctx: SkedgyContext, T: SkedgyHandler<Context = Ctx>> SkedgyTaskBuilder<Ctx, T> {
    pub fn named(id: &str) -> Self {
        Self {
            kind: None,
            handler: None,
            id: Some(id.to_string()),
        }
    }

    pub fn new() -> Self {
        Self {
            kind: None,
            handler: None,
            id: nanoid!(10).into(),
        }
    }

    pub fn at(&mut self, datetime: DateTime<Utc>) -> &mut Self {
        self.kind = Some(TaskKind::At(datetime));
        self
    }

    pub fn r#in(&mut self, duration: Duration) -> &mut Self {
        let datetime = Utc::now() + duration;
        self.kind = Some(TaskKind::At(datetime));
        self
    }

    pub fn cron(&mut self, pattern: &str) -> Result<&mut Self, SkedgyError> {
        let schedule = cron::Schedule::from_str(pattern).map_err(|_| SkedgyError::InvalidCron)?;
        self.kind = Some(TaskKind::Cron(schedule));
        Ok(self)
    }

    pub fn handler(&mut self, handler: T) -> &mut Self {
        self.handler = Some(handler);
        self
    }

    pub fn build(&mut self) -> Result<SkedgyTask<Ctx, T>, SkedgyError> {
        let kind = self.kind.clone().ok_or(SkedgyError::InvalidCron)?;
        let handler = self.handler.take().expect("Handler not set");
        let id = self.id.clone().unwrap_or_else(|| nanoid!(10));
        Ok(SkedgyTask { id, kind, handler })
    }
}
