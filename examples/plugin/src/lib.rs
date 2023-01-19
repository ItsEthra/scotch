use common::Object;
use scotch_guest::guest_function;

extern "C" {
    fn print_number(v: i32);
}

#[guest_function]
fn object_add_up(obj: Object) -> f32 {
    unsafe { print_number(obj.b) }

    obj.a + obj.b as f32
}
