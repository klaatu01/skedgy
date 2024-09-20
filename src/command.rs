use crate::{scheduler::DynSkedgyTask, SkedgyContext};

pub(crate) enum SkedgyCommand<Ctx: SkedgyContext> {
    Add(DynSkedgyTask<Ctx>),
    Remove(String),
    Update(DynSkedgyTask<Ctx>),
}
