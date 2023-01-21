scotch_guest::include_alloc!();

use scotch_guest::guest_function;

#[guest_function]
fn add_up_list(items: &Vec<i32>) -> i32 {
    items.iter().sum::<i32>()
}
