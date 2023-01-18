#![allow(dead_code)]

use crate::{WasmAllocator, WasmAllocatorOptions};
use parking_lot::RwLock;
use std::sync::Arc;
use wasmer::{CompileError, Imports, Instance, InstantiationError, Module, Store};

struct Managed {
    store: RwLock<Store>,
    instance: Instance,
    alloc: WasmAllocator,
}

pub struct WasmPlugin {
    managed: Arc<Managed>,
}

impl WasmPlugin {
    pub fn builder() -> WasmPluginBuilder {
        WasmPluginBuilder::default()
    }
}

#[derive(Default)]
pub struct WasmPluginBuilder {
    store: Store,
    module: Option<Module>,
    alloc_opts: WasmAllocatorOptions,
    imports: Option<Imports>,
}

impl WasmPluginBuilder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
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

    pub fn with_imports(mut self, imports: impl FnOnce(&mut Store) -> Imports) -> Self {
        self.imports = Some(imports(&mut self.store));
        self
    }

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
        let alloc = WasmAllocator::new(&mut self.store, &memory, self.alloc_opts)
            .expect("Failed to create allocator. Memory grow failed");

        let managed = Managed {
            store: self.store.into(),
            instance,
            alloc,
        };

        Ok(WasmPlugin {
            managed: Arc::new(managed),
        })
    }
}
