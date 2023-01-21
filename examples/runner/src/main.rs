use eyre::Result;
use scotch_host::{guest_functions, host_function, make_exports, make_imports, WasmPlugin};

const PLUGIN_BYTES: &[u8] = include_bytes!("../plugin.wasm");

guest_functions! {
    pub add_up_list: fn(nums: &Vec<i32>) -> i32;
}

#[host_function]
fn print(text: &String) {
    println!("Wasm: {text}");
}

fn main() -> Result<()> {
    let plugin = WasmPlugin::builder()
        .with_env(())
        .from_binary(PLUGIN_BYTES)?
        .with_imports(make_imports!(print))
        .with_exports(make_exports!(add_up_list))
        .finish()?;

    let sum = plugin.function::<add_up_list>()(&vec![1, 2, 3, 4, 5])?;
    assert_eq!(sum, 15);

    Ok(())
}
