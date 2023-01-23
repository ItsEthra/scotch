# Scotch
Library for creating WASM plugins with Rust.
Scotch allows you to pass complex types to/from functions in WASM plugins.
It achieves that by encoding and decoding complex types when passed between host and guest environment.
Encoding and decoding is handled by `bincode@2.0.0-rc.2` so you need your types
to implement `bincode::Encode` and `bincode::Decode` traits.

## Instalation
```toml
# In your main application
[dependenices]
scotch-host = "0.1"

# In your plugins
[dependencies]
scotch-guest = "0.1"
```

## Example application
```rust
// Define functions that your plugin exports.
#[scotch_host::guest_functions]
extern "C" {
    // The name must match with the name of the plugin function.
    pub fn add_up_list(nums: &Vec<i32>) -> i32;
}

// Create your plugin
let plugin = WasmPlugin::builder()
    .with_state(0)
    // PLUGIN_BYTES is a slice of your wasm plugin.
    .from_binary(PLUGIN_BYTES)?
    // This call caches exports of your plugin.
    .with_exports(make_exports![add_up_list])
    .finish()?;

// Call the function
let sum = plugin.function_unwrap::<add_up_list_renamed>()(&vec![1, 2, 3, 4, 5])?;
assert_eq!(sum, 15);
```

## Example plugin
```rust
// This is required.
scotch_guest::export_alloc!();

// Functions marked with `guest_function` will be exposed to the host.
// All complex types such as Vec, String, user-defined types must be passed by immutable reference.
#[scotch_guest::guest_function]
fn add_up_list(items: &Vec<i32>) -> i32 {
    items.iter().sum::<i32>()
}
```

More complete example can be found [here](/examples)
