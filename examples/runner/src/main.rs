use bincode::config::standard;
use common::Object;
use eyre::Result;
use scotch_host::EncodedString;
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
    println!("Add numbers: {}", add_numbers.call(&mut store, 10, 20)?);

    let add_object: TypedFunction<i32, f32> =
        instance.exports.get_typed_function(&store, "add_object")?;

    let object = Object { a: 3.5, b: 8 };

    let memory = instance.exports.get_memory("memory")?;
    let view = memory.view(&store);

    let mut data = bincode::encode_to_vec(&object, standard())?;
    data.insert(0, 5);
    data.insert(1, 0);
    view.write(0x200, &data[..])?;

    // 3.1 + 8 = 11.5
    println!("Add object: {}", add_object.call(&mut store, 0x200)?);

    let get_string: TypedFunction<i32, EncodedString> =
        instance.exports.get_typed_function(&store, "get_string")?;

    let out = get_string.call(&mut store, 13)?;
    let view = memory.view(&store);

    println!("Get string: {:?}", out.read(&view)?);

    Ok(())
}
