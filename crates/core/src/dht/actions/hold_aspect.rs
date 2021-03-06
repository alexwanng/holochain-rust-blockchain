use crate::{
    action::{Action, ActionWrapper},
    context::Context,
    instance::dispatch_action,
};
use futures::{future::Future, task::Poll};
use holochain_core_types::{error::HolochainError, network::entry_aspect::EntryAspect};
use snowflake::ProcessUniqueId;
use std::{pin::Pin, sync::Arc};

pub async fn hold_aspect(aspect: EntryAspect, context: Arc<Context>) -> Result<(), HolochainError> {
    let action_wrapper = ActionWrapper::new(Action::HoldAspect(aspect.clone()));
    dispatch_action(context.action_channel(), action_wrapper.clone());
    let id = ProcessUniqueId::new();
    HoldAspectFuture {
        context,
        aspect,
        id,
    }
    .await
}

pub struct HoldAspectFuture {
    context: Arc<Context>,
    aspect: EntryAspect,
    id: ProcessUniqueId,
}

#[holochain_tracing_macros::newrelic_autotrace(HOLOCHAIN_CORE)]
impl Future for HoldAspectFuture {
    type Output = Result<(), HolochainError>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Self::Output> {
        if let Some(err) = self.context.action_channel_error("HoldAspectFuture") {
            return Poll::Ready(Err(err));
        }
        self.context
            .register_waker(self.id.clone(), cx.waker().clone());
        if let Some(state) = self.context.try_state() {
            // TODO: wait for it to show up in the holding list
            // i.e. once we write the reducer we'll know
            if state.dht().get_holding_map().contains(&self.aspect) {
                self.context.unregister_waker(self.id.clone());
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        } else {
            Poll::Pending
        }
    }
}
