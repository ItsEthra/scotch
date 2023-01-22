scotch_guest::include_alloc!();

#[cfg(not(bench))]
#[scotch_guest::host_functions]
extern "C" {
    fn print(val: &String);
    fn random_cat_fact() -> [String; 2];
}

#[scotch_guest::guest_function]
fn add_up_list(items: &Vec<i32>) -> i32 {
    // Print numbers in reverse because why not.
    #[cfg(not(bench))]
    items
        .iter()
        .rev()
        .map(|num| format!("Hello number, {num}"))
        .for_each(|text| print(&text));

    items.iter().sum::<i32>()
}

#[scotch_guest::guest_function]
fn greet(name: &String) -> String {
    let [fact1, fact2] = random_cat_fact();
    format!("Hello, {name}! Did you know that {fact1} and {fact2}")
}
