use eyre::Result;
use scotch_host::{guest_functions, make_exports, WasmPlugin};

const PLUGIN_BYTES: &[u8] = include_bytes!("../plugin.wasm");

guest_functions! {
    pub add_up_list: fn(nums: &Vec<i32>) -> i32;
}

fn main() -> Result<()> {
    let plugin = WasmPlugin::builder()
        .from_binary(PLUGIN_BYTES)?
        .with_exports(make_exports!(add_up_list))
        .with_env(0)
        .finish()?;

    let sum = plugin.function::<add_up_list>()(&vec![1, 2, 3, 4, 5])?;
    assert_eq!(sum, 15);

    Ok(())
}
