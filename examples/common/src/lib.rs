use bincode::{Decode, Encode};

#[derive(Debug, Clone, Encode, Decode)]
pub struct Object {
    pub a: f32,
    pub b: i32,
    pub t: u64,
    pub thing: String,
}
