use eyre::Result;
use scotch_host::{host_function, make_imports, Function, WasmPlugin};
use wasmer::FunctionEnvMut;

const PLUGIN: &[u8] = include_bytes!("../plugin.wasm");

mod other {
    use scotch_host::host_function;

    #[host_function]
    fn get_number(a: i32) -> i32 {
        a + 5
    }
}

#[host_function(Type)]
fn print_number(a: i32) {
    println!("Number from wasm: {a}");
}

fn expanded_fn(_env: FunctionEnvMut<()>, a: i32) {
    println!("Number from wasm: {a}");
}

fn main() -> Result<()> {
    let _plugin = WasmPlugin::builder()
        .with_env(())
        .with_imports(|store, env| {
            let f = Function::new_typed_with_env(store, env, expanded_fn);
            todo!()
        })
        .from_binary(PLUGIN)?
        .finish()?;

    Ok(())
}
