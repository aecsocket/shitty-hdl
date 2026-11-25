#[derive(Debug, Clone, Copy)]
pub struct Bit<const N: usize>([u64; bit_cap::<N>()])
where
    [(); bit_cap::<N>()]:;

#[must_use]
pub const fn bit_cap<const N: usize>() -> usize {
    N.div_ceil(64)
}
