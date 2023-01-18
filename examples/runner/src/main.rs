use eyre::Result;
use scotch_host::host_function;
use wasmer::{imports, Instance, Module, Store};

const PLUGIN: &[u8] = include_bytes!("../plugin.wasm");

#[host_function]
fn get_number(a: i32) -> i32 {
    a + 5
}

fn main() -> Result<()> {
    let mut store = Store::default();
    let module = Module::from_binary(&store, PLUGIN)?;

    let imports = imports! {};
    let _instance = Instance::new(&mut store, &module, &imports)?;

    Ok(())
}
