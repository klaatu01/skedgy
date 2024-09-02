use crate::{
    handler::SkedgyHandler,
    scheduler::{SkedgyState, SkedgyTask},
};

pub(crate) enum SkedgyCommand<T: SkedgyHandler> {
    Add(SkedgyTask<T>),
    Remove(String),
    Update(SkedgyTask<T>),
    GetState(async_channel::Sender<SkedgyState<T>>),
    LoadState(SkedgyState<T>),
}
