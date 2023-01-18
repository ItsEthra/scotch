use eyre::Result;
use wasmer::{imports, Instance, Module, Store};

const PLUGIN: &[u8] = include_bytes!("../plugin.wasm");

fn main() -> Result<()> {
    let mut store = Store::default();
    let module = Module::from_binary(&store, PLUGIN)?;

    let imports = imports! {};
    let _instance = Instance::new(&mut store, &module, &imports)?;

    Ok(())
}
