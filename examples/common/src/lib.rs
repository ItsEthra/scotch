use bincode::{Decode, Encode};

#[derive(Encode, Decode)]
pub struct Object {
    pub a: f32,
    pub b: i32,
}
