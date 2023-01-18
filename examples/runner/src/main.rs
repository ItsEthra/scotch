use eyre::Result;
use scotch_host::{
    host_function, make_imports, CallbackRef, GuestFunctionCreator, GuestFunctionHandle,
    RuntimeError, WasmPlugin,
};
use std::{any::TypeId, mem::transmute};
use wasmer::TypedFunction;

const PLUGIN: &[u8] = include_bytes!("../plugin.wasm");

mod other {
    use scotch_host::host_function;

    #[host_function]
    pub fn get_number(a: i32) -> i32 {
        a + 5
    }
}

#[host_function]
fn print_number(a: i32) {
    println!("Number from wasm: {a}");
}

fn main() -> Result<()> {
    struct AddNumberHandle;
    unsafe impl GuestFunctionHandle for AddNumberHandle {
        type Callback = Box<dyn Fn(i32) -> Result<i32, RuntimeError>>;
    }

    unsafe impl GuestFunctionCreator for AddNumberHandle {
        fn create(
            &self,
            store: scotch_host::StoreRef,
            exports: &scotch_host::Exports,
        ) -> (TypeId, CallbackRef) {
            let typedfn: TypedFunction<i32, i32> = exports
                .get_typed_function(&*store.read(), "add_number")
                .unwrap();

            let callback = Box::new(move |arg1: i32| -> Result<i32, RuntimeError> {
                typedfn.call(&mut *store.write(), arg1)
            }) as <Self as GuestFunctionHandle>::Callback;

            (TypeId::of::<AddNumberHandle>(), unsafe {
                transmute(callback)
            })
        }
    }

    let plugin = WasmPlugin::builder()
        .with_env(())
        .with_imports(make_imports![other::get_number, print_number])
        .with_exports(vec![
            Box::new(AddNumberHandle) as Box<dyn GuestFunctionCreator>
        ])
        .from_binary(PLUGIN)?
        .finish()?;

    _ = dbg!(plugin.function::<AddNumberHandle>()(5));

    Ok(())
}
