use common::Object;
use eyre::Result;
use scotch_host::{guest_functions, host_function, make_exports, make_imports, WasmPlugin};

const PLUGIN_BYTES: &[u8] = include_bytes!("../plugin.wasm");

#[guest_functions]
extern "C" {
    // The name must match with the name of the plugin function.
    #[link_name = "add_up_list_renamed"]
    pub fn add_up_list(nums: &Vec<i32>) -> i32;

    pub fn greet(name: &String) -> String;
    pub fn sum_object(obj: &Object) -> f32;
}

// `i32` is the state type. You can skip it if you are not using state.
#[host_function(i32)]
fn print(text: &String) {
    *state += 1;
    println!("Wasm: {text}. Call count: {state}");
}

const RANDOM_NUMBER_CHOSEN_BY_A_FAIR_DICE_ROLL: usize = 2;
const FACTS: [&str; 4] = [
    "Cats can jump up to 6 times their height",
    "1 year of a cats life equals to 15 years of a humans live",
    "Cats sleep for around 13 to 16 hours a day",
    "A cat can run up to 30mph",
];

fn get_random_cat_fact() -> [String; 2] {
    let fact1 = FACTS[RANDOM_NUMBER_CHOSEN_BY_A_FAIR_DICE_ROLL].to_owned();
    let fact2 = FACTS[RANDOM_NUMBER_CHOSEN_BY_A_FAIR_DICE_ROLL - 1].to_owned();

    [fact1, fact2]
}

#[host_function(i32)]
fn random_cat_fact() -> [String; 2] {
    get_random_cat_fact()
}

fn main() -> Result<()> {
    let plugin = WasmPlugin::builder()
        // Initial plugin state, host functions will have mutable access to it.
        .with_state(0)
        .from_binary(PLUGIN_BYTES)?
        // This makes `print` accessible to the plugin.
        .with_imports(make_imports![print, random_cat_fact])
        // This will cache `add_up_list` in plugin exports.
        // Not necessery but preferred.
        .with_exports(make_exports![add_up_list_renamed, greet, sum_object])
        .finish()?;

    // If we had't call `.with_exports(make_exports![add_up_list])` this would fail.
    let sum = plugin.function_unwrap::<add_up_list_renamed>()(&vec![1, 2, 3, 4, 5])?;
    // You can use this to cache and get functions you hadn't cached using `.with_exports`.
    // let sum = plugin.function_unwrap_or_cache::<add_up_list>()(&vec![1, 2, 3, 4, 5])?;
    assert_eq!(sum, 15);

    // This shows that guest functions can also return complex types.
    let welcome = plugin.function_unwrap::<greet>()(&"Jack".into())?;

    let [fact1, fact2] = get_random_cat_fact();
    assert_eq!(
        welcome,
        format!("Hello, Jack! Did you know that {fact1} and {fact2}")
    );

    let result = plugin.function_unwrap::<sum_object>()(&Object {
        first: 5.3,
        second: 10,
        text: "Some text".to_owned(),
    })?;
    assert_eq!(result, 15.3);

    Ok(())
}
