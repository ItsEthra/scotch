use common::Object;
use eyre::Result;
use scotch_host::{guest_functions, host_function, make_exports, make_imports, WasmPlugin};

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

guest_functions! {
    pub add_number as AddNumber => fn(named: i32) -> f32,
    pub add_all => fn(obj: Object) -> f32
}

fn main() -> Result<()> {
    let plugin = WasmPlugin::builder()
        .with_env(())
        .with_imports(make_imports![other::get_number, print_number])
        .with_exports(make_exports![AddNumber, add_all])
        .from_binary(PLUGIN)?
        .finish()?;

    let val = plugin.function::<AddNumber>()(15)?;
    dbg!(val);

    let all = plugin.function::<add_all>()(Object { a: 5.3, b: 4 })?;

    dbg!(all);

    Ok(())
}
