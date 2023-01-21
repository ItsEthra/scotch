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

pub struct WasmEnv<S: PluginState> {
    pub instance: Weak<Instance>,
    pub state: S,
}

pub trait PluginState: Any + Send + 'static + Sized {}
impl<T> PluginState for T where T: Any + Send + 'static + Sized {}

#[allow(dead_code)]
pub struct WasmPlugin {
    exports: HashMap<TypeId, CallbackRef>,
    store: StoreRef,
    module: Module,
    instance: InstanceRef,
}

impl WasmPlugin {
    pub fn builder<E: PluginState>() -> WasmPluginBuilder<E> {
        WasmPluginBuilder::new()
    }

    pub fn function<H: GuestFunctionHandle + 'static>(&self) -> &H::Callback {
        self.exports
            .get(&TypeId::of::<H>())
            .expect("Export not found")
            .downcast_ref::<H::Callback>()
            .unwrap()
    }

    pub fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        self.module.serialize().map(|bytes| bytes.to_vec())
    }

    pub fn serialize_to_file(&self, path: impl AsRef<Path>) -> Result<(), SerializeError> {
        self.module.serialize_to_file(path)
    }
}

pub struct WasmPluginBuilder<E: PluginState> {
    store: Store,
    module: Option<Module>,
    imports: Option<Imports>,
    exports: Vec<Box<dyn GuestFunctionCreator>>,
    func_env: Option<FunctionEnv<WasmEnv<E>>>,
}

impl<E: PluginState> WasmPluginBuilder<E> {
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

    pub fn new_with_store(store: Store) -> Self {
        Self {
            store,
            ..Self::new()
        }
    }

    #[cfg(feature = "compiler")]
    pub fn from_binary(mut self, bytecode: &[u8]) -> Result<Self, CompileError> {
        self.module = Some(Module::from_binary(&self.store, bytecode)?);
        Ok(self)
    }

    pub unsafe fn from_serialized(mut self, data: &[u8]) -> Result<Self, DeserializeError> {
        self.module = Some(Module::deserialize(&self.store, data)?);
        Ok(self)
    }

    pub unsafe fn from_serialized_file(
        mut self,
        path: impl AsRef<Path>,
    ) -> Result<Self, DeserializeError> {
        self.module = Some(Module::deserialize_from_file(&self.store, path)?);
        Ok(self)
    }

    pub fn with_env(mut self, env: E) -> Self {
        self.func_env = Some(FunctionEnv::new(
            &mut self.store,
            WasmEnv {
                instance: Weak::new(),
                state: env,
            },
        ));
        self
    }

    pub fn with_imports(
        mut self,
        imports: impl FnOnce(&mut Store, &FunctionEnv<WasmEnv<E>>) -> Imports,
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
            .map(|ex| ex.create(store.clone(), instance.clone(), &instance.exports))
            .collect::<HashMap<_, _>>();

        Ok(WasmPlugin {
            store,
            exports,
            instance,
            module,
        })
    }
}

impl<E: PluginState> Default for WasmPluginBuilder<E> {
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
