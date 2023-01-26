use bincode::{Decode, Encode};

#[derive(Debug, Clone, Encode, Decode)]
pub struct Object {
    pub first: f32,
    pub second: i32,
    pub text: String,
}
