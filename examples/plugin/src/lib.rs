use common::Object;
use scotch_guest::{guest_function, host_functions};

#[host_functions]
extern "C" {
    fn print_number(v: i32);
}

#[guest_function]
fn object_add_up(obj: Object, things: (i32, f64)) -> f32 {
    print_number(obj.b);
    print_number(things.0);

    obj.a + obj.b as f32
}
