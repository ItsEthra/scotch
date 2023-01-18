#![allow(dead_code)]

use crate::{
    CallbackRef, EncodedPtr, GuestFunctionCreator, GuestFunctionHandle, WasmAllocator,
    WasmAllocatorOptions,
};
use bincode::{config::standard, Decode, Encode};
use parking_lot::RwLock;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    mem::{size_of, transmute},
    sync::Arc,
};
use wasmer::{
    CompileError, Extern, FunctionEnv, Imports, Instance, InstantiationError, MemoryAccessError,
    Module, Store,
};

pub trait WasmEnv: Any + Send + 'static + Sized {}
impl<T> WasmEnv for T where T: Any + Send + 'static + Sized {}

pub struct WasmPlugin {
    exports: HashMap<TypeId, CallbackRef>,
    store: Arc<RwLock<Store>>,
    alloc: WasmAllocator,
    instance: Instance,
}

impl WasmPlugin {
    pub fn builder<E: WasmEnv>() -> WasmPluginBuilder<E> {
        WasmPluginBuilder::new()
    }

    pub fn new_encoded<T: Encode + Decode>(
        &self,
        value: T,
    ) -> Result<EncodedPtr<T>, MemoryAccessError> {
        let mut buf = [0u8; 256];
        let view = self
            .instance
            .exports
            .get_memory("memory")
            .unwrap()
            .view(&*self.store.read());

        type PrefixType = u16;

        // First try encoding to the stack if the object is small,
        // otherwise encode to the heap.
        if let Ok(size) = bincode::encode_into_slice(value, &mut buf[..], standard()) {
            let ptr = self
                .alloc
                .alloc((size + size_of::<PrefixType>()) as u32)
                .expect("Allocation failed");
            view.write(ptr as u64, &(size as PrefixType).to_le_bytes())?;
            view.write(ptr as u64 + size_of::<PrefixType>() as u64, &buf[..size])?;

            Ok(EncodedPtr::new(ptr))
        } else {
            todo!()
        }
    }

    pub fn function<H: GuestFunctionHandle + 'static>(&self) -> &H::Callback {
        let export = self.exports.get(&TypeId::of::<H>()).unwrap();
        unsafe { transmute(export) }
    }
}

pub struct WasmPluginBuilder<E: WasmEnv> {
    store: Store,
    module: Option<Module>,
    alloc_opts: WasmAllocatorOptions,
    imports: Option<Imports>,
    exports: Vec<Box<dyn GuestFunctionCreator>>,
    func_env: Option<FunctionEnv<E>>,
}

impl<E: WasmEnv> WasmPluginBuilder<E> {
    #[inline]
    pub fn new() -> Self {
        Self {
            store: Store::default(),
            module: None,
            alloc_opts: WasmAllocatorOptions::default(),
            imports: None,
            func_env: None,
            exports: vec![],
        }
    }

    pub fn new_with_store(store: Store) -> Self {
        Self {
            store,
            ..Self::new()
        }
    }

    pub fn from_binary(mut self, wasm: &[u8]) -> Result<Self, CompileError> {
        self.module = Some(Module::from_binary(&self.store, wasm)?);
        Ok(self)
    }

    pub fn with_alloc_opts(mut self, alloc_opts: WasmAllocatorOptions) -> Self {
        self.alloc_opts = alloc_opts;
        self
    }

    pub fn with_env(mut self, env: E) -> Self {
        self.func_env = Some(FunctionEnv::new(&mut self.store, env));
        self
    }

    pub fn with_imports(
        mut self,
        imports: impl FnOnce(&mut Store, &FunctionEnv<E>) -> Imports,
    ) -> Self {
        self.imports = Some(imports(
            &mut self.store,
            self.func_env
                .as_ref()
                .expect("You need to call `with_env` first"),
        ));
        self
    }

    pub fn with_exports(
        mut self,
        exports: impl IntoIterator<Item = Box<dyn GuestFunctionCreator>>,
    ) -> Self {
        self.exports.extend(exports);
        self
    }

    #[allow(clippy::result_large_err)]
    pub fn finish(mut self) -> Result<WasmPlugin, InstantiationError> {
        let instance = Instance::new(
            &mut self.store,
            self.module
                .as_ref()
                .expect("You need to call `from_binary` first"),
            &self.imports.unwrap_or_default(),
        )?;

        let memory = instance
            .exports
            .get_memory("memory")
            .expect("Memory is not found. Expected `memory` export.");
        let alloc = WasmAllocator::new(&mut self.store, memory, self.alloc_opts)
            .expect("Failed to create allocator. Memory grow failed");

        let store: Arc<RwLock<Store>> = Arc::new(self.store.into());

        let exports = self
            .exports
            .into_iter()
            .map(|ex| ex.create(store.clone(), &instance.exports))
            .collect::<HashMap<_, _>>();

        Ok(WasmPlugin {
            store,
            exports,
            instance,
            alloc,
        })
    }
}

impl<E: WasmEnv> Default for WasmPluginBuilder<E> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

pub use wasmer::{Function, FunctionEnvMut};
pub fn create_imports_from_functions<const N: usize>(
    items: [(&'static str, Function); N],
) -> Imports {
    let mut imports = Imports::new();
    imports.register_namespace(
        "env",
        items
            .into_iter()
            .map(|(s, f)| (s.to_string(), Extern::Function(f))),
    );
    imports
}
