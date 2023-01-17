use common::Object;

extern "C" {
    fn get_number() -> i32;
}

#[no_mangle]
extern "C" fn add_numbers(a: i32, b: i64) -> i64 {
    unsafe { a as i64 + b + get_number() as i64 }
}

#[no_mangle]
extern "C" fn add_object(obj: &Object) -> f32 {
    obj.b as f32 + obj.a
}
