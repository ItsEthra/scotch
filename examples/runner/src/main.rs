use eyre::Result;
use scotch_host::{guest_functions, host_function, make_exports, make_imports, WasmPlugin};

const PLUGIN_BYTES: &[u8] = include_bytes!("../plugin.wasm");

#[guest_functions]
extern "C" {
    // The name must match with the name of the plugin function.
    #[link_name = "hehe"]
    pub fn add_up_list(nums: &Vec<i32>) -> i32;
}

// `i32` is the state type. You can skip it if you are not using state.
#[host_function(i32)]
fn print(text: &String) {
    *STATE += 1;
    println!("Wasm: {text}. Call count: {STATE}");
}

fn main() -> Result<()> {
    let plugin = WasmPlugin::builder()
        .with_state(0)
        .from_binary(PLUGIN_BYTES)?
        // This makes `print` accessible to the plugin.
        .with_imports(make_imports!(print))
        // This will cache `add_up_list` in plugin exports.
        // Not necessery but preferred.
        .with_exports(make_exports!(hehe))
        .finish()?;

    // If we had't call `.with_exports(make_exports!(add_up_list))` this would fail.
    let sum = plugin.function_unwrap::<hehe>()(&vec![1, 2, 3, 4, 5])?;
    // You can use this to cache and get functions you hadn't cached using `.with_exports`.
    // let sum = plugin.function_unwrap_or_cache::<add_up_list>()(&vec![1, 2, 3, 4, 5])?;
    assert_eq!(sum, 15);

    Ok(())
}
