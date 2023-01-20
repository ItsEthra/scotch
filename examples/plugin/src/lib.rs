scotch_guest::include_alloc!();

use common::Object;
use scotch_guest::{guest_function, host_functions};

#[host_functions]
extern "C" {
    fn print_number(v: i32);
    fn accept_object(v: &Object);
}

#[guest_function]
fn object_add_up(obj: &Object) -> f32 {
    print_number(obj.b);

    accept_object(obj);

    obj.a + obj.b as f32
}
