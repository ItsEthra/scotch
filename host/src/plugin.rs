use crate::{CallbackRef, GuestFunctionCreator, GuestFunctionHandle, InstanceRef, StoreRef};
use std::{
    any::{Any, TypeId},
    collections::{hash_map::Entry, HashMap},
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

    /// Looks up cached guest export by function handle.
    /// If no matches are found tries to resolve export from wasm instance and cache the result.
    pub fn function_or_cache<H: GuestFunctionHandle + 'static>(&mut self) -> Option<&H::Callback> {
        let type_id = TypeId::of::<H>();

        if let Entry::Vacant(e) = self.exports.entry(type_id) {
            let callback = H::new()
                .create(self.store.clone(), self.instance.clone())?
                .1;
            e.insert(callback);
        }

        self.exports.get(&type_id).and_then(|f| f.downcast_ref())
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

    /// Looks up cached guest export by function handle.
    /// If no matches are found tries to resolve export from wasm instance and cache the result.
    /// # Panics
    /// If failed to find function in exports and it is missing in wasm instance.
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

    /// Serializes plugin and compresses bytes to use with headless mode.
    #[cfg(feature = "flate2")]
    #[cfg_attr(feature = "unstable-doc-cfg", doc(cfg(feature = "flate2")))]
    pub fn serialize_compress(&self) -> Result<Vec<u8>, SerializeError> {
        use flate2::Compression;
        use std::io::Write;

        let data = self.serialize()?;
        let mut encoder = flate2::write::GzEncoder::new(vec![], Compression::best());
        encoder.write_all(&data[..])?;

        Ok(encoder.finish()?)
    }

    /// Serializes plugin to file and compresses bytes to use with headless mode.
    #[cfg(feature = "flate2")]
    #[cfg_attr(feature = "unstable-doc-cfg", doc(cfg(feature = "flate2")))]
    pub fn serialize_to_file_compress(&self, path: impl AsRef<Path>) -> Result<(), SerializeError> {
        let compressed = self.serialize_compress()?;
        Ok(std::fs::write(path, compressed)?)
    }
}

/// Builder for creating [`WasmPlugin`].
pub struct WasmPluginBuilder<E: Any + Send + Sized + 'static> {
    store: Store,
    module: Option<Module>,
    imports: Option<Imports>,
    exports: Vec<Box<dyn GuestFunctionCreator>>,
    func_env: Option<FunctionEnv<WasmEnv<E>>>,
}

impl<S: Any + Send + Sized + 'static> WasmPluginBuilder<S> {
    /// Creates new [`WasmPluginBuilder`].
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
    #[cfg_attr(feature = "unstable-doc-cfg", doc(cfg(feature = "compiler")))]
    pub fn from_binary(mut self, bytecode: &[u8]) -> Result<Self, CompileError> {
        self.module = Some(Module::from_binary(&self.store, bytecode)?);
        Ok(self)
    }

    /// Creates plugin from bytes created by [`WasmPlugin::serialize`].
    /// # Safety
    /// See [`Module::deserialize`].
    pub unsafe fn from_serialized(mut self, data: &[u8]) -> Result<Self, DeserializeError> {
        self.module = Some(Module::deserialize(&self.store, data)?);
        Ok(self)
    }

    /// Creates plugin from compressed bytes created by [`WasmPlugin::serialize_compress`].
    /// # Safety
    /// See [`Module::deserialize`].
    #[cfg(feature = "flate2")]
    #[cfg_attr(feature = "unstable-doc-cfg", doc(cfg(feature = "flate2")))]
    pub unsafe fn from_serialized_compressed(
        mut self,
        compressed: &[u8],
    ) -> Result<Self, DeserializeError> {
        use std::io::Read;

        let mut decoder = flate2::read::GzDecoder::new(compressed);
        let mut buf = vec![];
        decoder.read_to_end(&mut buf)?;

        self.module = Some(Module::deserialize(&self.store, buf)?);
        Ok(self)
    }

    /// Creates plugin from bytes created by [`WasmPlugin::serialize_to_file`].
    /// # Safety
    /// See [`Module::deserialize_from_file`].
    pub unsafe fn from_serialized_file(
        mut self,
        path: impl AsRef<Path>,
    ) -> Result<Self, DeserializeError> {
        self.module = Some(Module::deserialize_from_file(&self.store, path)?);
        Ok(self)
    }

    /// Creates plugin from compressed bytes created by [`WasmPlugin::serialize_to_file_compress`].
    /// # Safety
    /// See [`Module::deserialize`].
    #[cfg(feature = "flate2")]
    #[cfg_attr(feature = "unstable-doc-cfg", doc(cfg(feature = "flate2")))]
    pub unsafe fn from_serialized_file_compressed(
        mut self,
        path: impl AsRef<Path>,
    ) -> Result<Self, DeserializeError> {
        use std::io::Read;

        let compressed = std::fs::read(path)?;
        let mut decoder = flate2::read::GzDecoder::new(&compressed[..]);
        let mut buf = vec![];
        decoder.read_to_end(&mut buf)?;

        self.module = Some(Module::deserialize(&self.store, buf)?);
        Ok(self)
    }

    /// Creates a state that host function will have mutable access to.
    /// You *HAVE* to create the state. If you do not need it simply pass `()`.
    pub fn with_state(mut self, state: S) -> Self {
        // This should help avoid questionable bugs in `with_imports`
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
        instance
            .exports
            .get_memory("memory")
            .unwrap()
            .grow(&mut self.store, 3)
            .unwrap();

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
