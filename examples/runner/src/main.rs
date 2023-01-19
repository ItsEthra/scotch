use common::Object;
use eyre::Result;
use scotch_host::{guest_functions, host_function, make_exports, make_imports, WasmPlugin};

const PLUGIN_BYTES: &[u8] = include_bytes!("../plugin.wasm");

/* pub struct Object {
    pub a: f32,
    pub b: i32,
} */
guest_functions! {
    pub object_add_up as ObjectAddUp: fn(obj: Object) -> f32
}

#[host_function]
fn print_number(value: i32) {
    println!("Print from wasm: {value}");
}

fn main() -> Result<()> {
    let plugin = WasmPlugin::builder()
        .with_env(())
        .with_imports(make_imports![print_number])
        .with_exports(make_exports![ObjectAddUp])
        .from_binary(PLUGIN_BYTES)?
        .finish()?;

    let val = plugin.function::<ObjectAddUp>()(Object { a: 5.3, b: 4 })?;
    assert_eq!(val, 9.3);

    println!("Success");

    Ok(())
}
