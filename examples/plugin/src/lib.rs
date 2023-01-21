scotch_guest::include_alloc!();

use scotch_guest::{guest_function, host_functions};

#[host_functions]
extern "C" {
    fn print(val: &String);
}

#[guest_function]
fn add_up_list(items: &Vec<i32>) -> i32 {
    // Print numbers in reverse because why not.
    items
        .iter()
        .rev()
        .map(|num| format!("Hello number, {num}"))
        .for_each(|text| print(&text));

    items.iter().sum::<i32>()
}
