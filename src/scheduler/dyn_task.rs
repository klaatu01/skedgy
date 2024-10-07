use crate::{
    handler::{DynHandler, SkedgyHandler},
    task::{SkedgyTask, TaskKind},
    Metadata, SkedgyContext,
};

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
