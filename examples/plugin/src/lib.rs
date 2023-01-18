use common::Object;
use scotch_guest::EncodedPtr;

extern "C" {
    fn get_number(v: i32) -> i32;
    fn print_number(v: i32);
}

#[no_mangle]
pub extern "C" fn add_number(a: i32) -> f32 {
    unsafe {
        print_number(a);
        a as f32 + get_number(a) as f32
    }
}

#[no_mangle]
pub extern "C" fn add_all(obj: EncodedPtr<Object>) -> f32 {
    let obj = unsafe { obj.read().unwrap() };
    obj.a + obj.b as f32
}
