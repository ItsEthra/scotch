use common::Object;
use eyre::Result;
use scotch_host::{guest_functions, host_function, make_exports, make_imports, WasmPlugin};

const PLUGIN_BYTES: &[u8] = include_bytes!("../plugin.wasm");

/* pub struct Object {
    pub a: f32,
    pub b: i32,
} */
guest_functions! {
    pub object_add_up as ObjectAddUp: fn(obj: &Object) -> f32;
}

#[host_function(i32)]
fn print_number(value: i32) {
    *STATE += 1;
    dbg!(*STATE);
    println!("Print from wasm: {value}");
}

#[host_function(i32)]
fn accept_object(obj: &Object) {
    dbg!(obj);
}

fn main() -> Result<()> {
    let plugin = WasmPlugin::builder()
        .with_env(0)
        .with_imports(make_imports![print_number, accept_object])
        .with_exports(make_exports![ObjectAddUp])
        .from_binary(PLUGIN_BYTES)?
        .finish()?;

    dbg!(plugin.function::<ObjectAddUp>()(&Object {
        thing: "123".into(),
        a: 123.5,
        b: 11,
        t: 5,
    })?);

    Ok(())
}
