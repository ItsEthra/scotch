use parking_lot::RwLock;
use std::{
    any::{Any, TypeId},
    sync::Arc,
};

pub use wasmer::{Exports, Instance, RuntimeError, Store, TypedFunction};

pub type StoreRef = Arc<RwLock<Store>>;
pub type InstanceRef = Arc<Instance>;

pub type CallbackRef = Box<dyn Any>;

/// # Safety
/// Do not implemented this trait manually.
pub unsafe trait GuestFunctionHandle {
    type Callback;
}

/// # Safety
/// Do not implemented this trait manually.
pub unsafe trait GuestFunctionCreator {
    fn create(
        &self,
        store: StoreRef,
        instance: InstanceRef,
        exports: &Exports,
    ) -> (TypeId, CallbackRef);
}
