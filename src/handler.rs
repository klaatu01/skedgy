use chrono::{DateTime, Utc};
use dyn_clone::DynClone;
use futures::{future::BoxFuture, FutureExt};

use crate::context::SkedgyContext;

#[derive(Debug, Clone)]
pub struct Metadata {
    pub id: String,
    pub target_time: DateTime<Utc>,
}

/// A trait for defining task handlers that can be scheduled by the `Skedgy` scheduler.
/// Implement this trait for your task handler and define the task's behavior in the `handle` method.
pub trait SkedgyHandler: Clone + Send + Sync + 'static {
    type Context: SkedgyContext;
    fn handle(
        &self,
        ctx: &Self::Context,
        metadata: Metadata,
    ) -> impl std::future::Future<Output = ()> + Send;
}

/// A trait for implementing a dynamic handler;
pub(crate) trait DynHandler<Ctx: SkedgyContext>: DynClone
where
    Self: Send,
{
    fn handle_dyn(self: Box<Self>, ctx: &Ctx, metadata: Metadata) -> BoxFuture<'_, ()>;
}

impl<Ctx: SkedgyContext, T: SkedgyHandler<Context = Ctx>> DynHandler<Ctx> for T {
    fn handle_dyn(self: Box<Self>, ctx: &Ctx, metadata: Metadata) -> BoxFuture<'_, ()> {
        async move { SkedgyHandler::handle(&*self, ctx, metadata).await }.boxed()
    }
}

dyn_clone::clone_trait_object!(<Ctx> DynHandler<Ctx>);
