use parking_lot::RwLock;
use std::{any::TypeId, sync::Arc};

pub use wasmer::{Exports, Store};

pub type StoreRef = Arc<RwLock<Store>>;

pub trait GuestFunctionHandle {
    type Callback;
}

pub trait GuestFunctionCreator {
    fn create(&self, store: StoreRef, exports: &Exports) -> (TypeId, u128);
}
