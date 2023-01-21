use crate::{CallbackRef, GuestFunctionCreator, GuestFunctionHandle, InstanceRef, StoreRef};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    path::Path,
    sync::{Arc, Weak},
};
use wasmer::{
    CompileError, DeserializeError, Extern, FunctionEnv, Imports, Instance, InstantiationError,
    Module, SerializeError, Store,
};

#[doc(hidden)]
pub struct WasmEnv<S: Any + Send + Sized + 'static> {
    pub instance: Weak<Instance>,
    pub state: S,
}

/// An instantiated plugin with cached exports.
#[allow(dead_code)]
pub struct WasmPlugin {
    exports: HashMap<TypeId, CallbackRef>,
    store: StoreRef,
    module: Module,
    instance: InstanceRef,
}

impl WasmPlugin {
    /// Creates a builder to create a new WasmPlugin.
    pub fn builder<E: Any + Send + Sized + 'static>() -> WasmPluginBuilder<E> {
        WasmPluginBuilder::new()
    }

    /// Looks up cached guest export by function handle.
    pub fn function<H: GuestFunctionHandle + 'static>(&self) -> Option<&H::Callback> {
        self.exports
            .get(&TypeId::of::<H>())?
            .downcast_ref::<H::Callback>()
    }

    pub fn function_or_cache<H: GuestFunctionHandle + 'static>(&mut self) -> Option<&H::Callback> {
        let type_id = TypeId::of::<H>();

        if !self.exports.contains_key(&type_id) {
            let callback = H::new()
                .create(self.store.clone(), self.instance.clone())?
                .1;
            self.exports.insert(type_id, callback);
            self.exports.get(&type_id).and_then(|f| f.downcast_ref())
        } else {
            self.exports.get(&type_id)?.downcast_ref::<H::Callback>()
        }
    }

    /// Looks up cached guest export by function handle.
    /// # Panics
    /// If function was not cached with `make_exports!`.
    pub fn function_unwrap<H: GuestFunctionHandle + 'static>(&self) -> &H::Callback {
        self.exports
            .get(&TypeId::of::<H>())
            .expect("Function not found")
            .downcast_ref::<H::Callback>()
            .unwrap()
    }

    pub fn function_unwrap_or_cache<'this: 'cb, 'cb, H: GuestFunctionHandle + 'static>(
        &'this mut self,
    ) -> &'cb H::Callback {
        let type_id = TypeId::of::<H>();

        self.exports
            .entry(type_id)
            .or_insert_with(|| {
                H::new()
                    .create(self.store.clone(), self.instance.clone())
                    .expect("Function not found")
                    .1
            })
            .downcast_ref()
            .unwrap()
    }

    /// Serializes plugin into bytes to use with headless mode.
    pub fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        self.module.serialize().map(|bytes| bytes.to_vec())
    }

    /// Serializes plugin into bytes to use with headless mode and writes them to file.
    pub fn serialize_to_file(&self, path: impl AsRef<Path>) -> Result<(), SerializeError> {
        self.module.serialize_to_file(path)
    }
}

/// Builder for creating [`WasmPlugin`]
pub struct WasmPluginBuilder<E: Any + Send + Sized + 'static> {
    store: Store,
    module: Option<Module>,
    imports: Option<Imports>,
    exports: Vec<Box<dyn GuestFunctionCreator>>,
    func_env: Option<FunctionEnv<WasmEnv<E>>>,
}

impl<S: Any + Send + Sized + 'static> WasmPluginBuilder<S> {
    /// Creates new [`WasmPluginBuilder`]
    #[inline]
    pub fn new() -> Self {
        Self {
            store: Store::default(),
            module: None,
            imports: None,
            func_env: None,
            exports: vec![],
        }
    }

    /// Creates new [`WasmPluginBuilder`] and overrides default store with custom.
    pub fn new_with_store(store: Store) -> Self {
        Self {
            store,
            ..Self::new()
        }
    }

    /// Compiles bytecode with selected compiler. To change the compile use feature flags.
    /// Default compiler is `cranelift`.
    #[cfg(feature = "compiler")]
    pub fn from_binary(mut self, bytecode: &[u8]) -> Result<Self, CompileError> {
        self.module = Some(Module::from_binary(&self.store, bytecode)?);
        Ok(self)
    }

    /// # Safety
    /// See [`Module::deserialize`]
    pub unsafe fn from_serialized(mut self, data: &[u8]) -> Result<Self, DeserializeError> {
        self.module = Some(Module::deserialize(&self.store, data)?);
        Ok(self)
    }

    /// # Safety
    /// See [`Module::deserialize_from_file`]
    pub unsafe fn from_serialized_file(
        mut self,
        path: impl AsRef<Path>,
    ) -> Result<Self, DeserializeError> {
        self.module = Some(Module::deserialize_from_file(&self.store, path)?);
        Ok(self)
    }

    /// Creates a state that host function will have mutable access to.
    /// You *HAVE* to create the state. If you do not need it simply pass `()`.
    pub fn with_state(mut self, state: S) -> Self {
        // This should help avoid questionable bugs in `with_imports`.
        assert!(
            self.func_env.is_none(),
            "You can call `with_state` only once"
        );

        self.func_env = Some(FunctionEnv::new(
            &mut self.store,
            WasmEnv {
                instance: Weak::new(),
                state,
            },
        ));
        self
    }

    /// Creates imports i.e. host functions that guest imports.
    /// use `make_imports!` to create the closure.
    pub fn with_imports(
        mut self,
        imports: impl FnOnce(&mut Store, &FunctionEnv<WasmEnv<S>>) -> Imports,
    ) -> Self {
        self.imports = Some(imports(
            &mut self.store,
            self.func_env
                .as_ref()
                .expect("You need to call `with_state` first"),
        ));
        self
    }

    /// Updates exports i.e. guest functions that host imports.
    /// use `make_exports!` to create the iterator.
    pub fn with_exports(
        mut self,
        exports: impl IntoIterator<Item = Box<dyn GuestFunctionCreator>>,
    ) -> Self {
        self.exports.extend(exports);
        self
    }

    /// Finishes building a `WasmPlugin`.
    #[allow(clippy::result_large_err)]
    pub fn finish(mut self) -> Result<WasmPlugin, InstantiationError> {
        let module = self
            .module
            .expect("You need to call `from_binary` or `from_serialized` first");
        let instance: InstanceRef =
            Instance::new(&mut self.store, &module, &self.imports.unwrap_or_default())?.into();

        if let Some(env) = self.func_env.as_mut() {
            env.as_mut(&mut self.store).instance = Arc::downgrade(&instance);
        }

        let store: StoreRef = Arc::new(self.store.into());
        let exports = self
            .exports
            .into_iter()
            .flat_map(|ex| ex.create(store.clone(), instance.clone()))
            .collect::<HashMap<_, _>>();

        Ok(WasmPlugin {
            store,
            exports,
            instance,
            module,
        })
    }
}

impl<E: Any + Send + Sized + 'static> Default for WasmPluginBuilder<E> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[doc(hidden)]
pub use wasmer::{Function, FunctionEnvMut};

#[doc(hidden)]
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
