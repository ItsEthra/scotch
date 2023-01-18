use eyre::Result;
use scotch_host::{guest_functions, host_function, make_imports, WasmPlugin};

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
    pub add_number => fn(named: i32) -> i32
}

fn main() -> Result<()> {
    let plugin = WasmPlugin::builder()
        .with_env(())
        .with_imports(make_imports![other::get_number, print_number])
        .with_exports(vec![
            Box::new(AddNumberHandle) as Box<dyn scotch_host::GuestFunctionCreator>
        ])
        .from_binary(PLUGIN)?
        .finish()?;

    let val = plugin.function::<AddNumberHandle>()(15)?;
    dbg!(val);

    Ok(())
}
