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
    print_number(obj.a as i32);

    accept_object(obj);

    obj.a + obj.b as f32
}

#[guest_function]
fn print_numbers(things: &Vec<i32>) {
    things.iter().copied().for_each(print_number);
}
