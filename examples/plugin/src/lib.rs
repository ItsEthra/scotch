use common::Object;
use scotch_guest::{EncodedPtr, EncodedString};

extern "C" {
    fn get_number() -> i32;
}

#[no_mangle]
extern "C" fn add_numbers(a: i32, b: i64) -> i64 {
    unsafe { a as i64 + b + get_number() as i64 }
}

#[no_mangle]
extern "C" fn add_object(obj: EncodedPtr<Object>) -> f32 {
    let obj = obj.read().unwrap();

    obj.a + obj.b as f32
}

#[no_mangle]
extern "C" fn get_string(number: i32) -> EncodedString {
    format!("Your number is: {number}").into()
}

#[no_mangle]
extern "C" fn capitalize(string: EncodedString) -> EncodedString {
    string.read().unwrap().to_uppercase().into()
}
