extern "C" {
    fn get_number(v: i32) -> i32;
    fn print_number(v: i32);
}

#[no_mangle]
pub extern "C" fn add_number(a: i32) -> i32 {
    unsafe {
        print_number(a);
        a + get_number(a)
    }
}
