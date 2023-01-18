use common::Object;
use scotch_guest::EncodedPtr;

extern "C" {
    fn print_number(v: i32);
}

#[no_mangle]
pub extern "C" fn object_add_up(obj: EncodedPtr<Object>) -> f32 {
    let obj = unsafe { obj.read().unwrap() };
    unsafe {
        print_number(obj.b);
    }

    obj.a + obj.b as f32
}
