use eyre::Result;
use wasmer::{imports, Function, Instance, Module, Store, TypedFunction};

const PLUGIN: &[u8] = include_bytes!("../plugin.wasm");

fn get_number() -> i32 {
    15
}

fn main() -> Result<()> {
    let mut store = Store::default();
    let module = Module::from_binary(&store, PLUGIN)?;

    let imports = imports! {
        "env" => {
            "get_number" => Function::new_typed(&mut store, get_number)
        }
    };
    let instance = Instance::new(&mut store, &module, &imports)?;

    let add_numbers: TypedFunction<(i32, i64), i64> =
        instance.exports.get_typed_function(&store, "add_numbers")?;

    // 10 + 20 + 15 = 45
    println!("Result: {}", add_numbers.call(&mut store, 10, 20)?);

    let add_object: TypedFunction<i32, f32> =
        instance.exports.get_typed_function(&store, "add_object")?;

    Ok(())
}
