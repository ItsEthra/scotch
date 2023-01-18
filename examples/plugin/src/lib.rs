extern "C" {
    pub fn get_number(val: i32) -> i32;
}

#[no_mangle]
pub extern "C" fn add_number(a: i32) -> i32 {
    unsafe { a + get_number(a) }
}
