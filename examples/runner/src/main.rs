use eyre::Result;
use scotch_host::{host_function, make_imports, WasmPlugin};

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
    let _plugin = WasmPlugin::builder()
        .with_env(())
        .with_imports(make_imports![other::get_number, print_number])
        .from_binary(PLUGIN)?
        .finish()?;

    Ok(())
}
