use crate::value::ShapeSize;

pub struct Assert<const CONDITION: bool>;

pub trait True {}
impl True for Assert<true> {}

pub const fn fits(width: ShapeSize, value: u64) -> bool {
    if width >= 64 {
        true
    } else {
        value < (1u64 << width)
    }
}
