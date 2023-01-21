use parking_lot::RwLock;
use std::{
    any::{Any, TypeId},
    sync::Arc,
};

#[doc(hidden)]
pub use wasmer::{Exports, Instance, RuntimeError, Store, TypedFunction};

#[doc(hidden)]
pub type StoreRef = Arc<RwLock<Store>>;
#[doc(hidden)]
pub type InstanceRef = Arc<Instance>;
#[doc(hidden)]
pub type CallbackRef = Box<dyn Any>;

#[doc(hidden)]
/// Do not implemented this trait manually.
pub unsafe trait GuestFunctionHandle: GuestFunctionCreator {
    type Callback;
}

#[doc(hidden)]
/// Do not implemented this trait manually.
pub unsafe trait GuestFunctionCreator {
    fn new() -> Self
    where
        Self: Sized;

    fn create(&self, store: StoreRef, instance: InstanceRef) -> Option<(TypeId, CallbackRef)>;
}
