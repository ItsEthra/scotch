use common::Object;
use eyre::Result;
use scotch_host::{
    guest_functions, host_function, make_exports, make_imports, Instance, ManagedPtr, WasmPlugin,
};
use std::sync::Arc;

const PLUGIN_BYTES: &[u8] = include_bytes!("../plugin.wasm");

/* pub struct Object {
    pub a: f32,
    pub b: i32,
} */
guest_functions! {
    pub object_add_up as ObjectAddUp: fn(obj: &Object) -> f32;
}

#[host_function]
fn print_number(value: i32) {
    println!("Print from wasm: {value}");
}

#[host_function]
fn accept_object(obj: ManagedPtr<Object>) {
    let ins: Arc<Instance> = __env.data().instance.upgrade().unwrap();
    let view = ins.exports.get_memory("memory").unwrap().view(&__env);

    _ = dbg!(obj.read(&view));
}

fn main() -> Result<()> {
    let plugin = WasmPlugin::builder()
        .with_env(())
        .with_imports(make_imports![print_number, accept_object])
        .with_exports(make_exports![ObjectAddUp])
        .from_binary(PLUGIN_BYTES)?
        .finish()?;

    dbg!(plugin.function::<ObjectAddUp>()(&Object {
        a: 123.5,
        b: 11,
        t: 5
    })?);

    Ok(())
}
