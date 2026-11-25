use crate::value::ShapeSize;

pub const fn fits_unsigned(width: ShapeSize, value: u64) -> bool {
    if width >= 64 {
        true
    } else {
        value < (1u64 << width)
    }
}
