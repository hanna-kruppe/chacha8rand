use crate::RefillFn;

pub mod scalar;
pub mod simd128;

// The constant words in the first row of the initial state
const C0: u32 = u32::from_le_bytes(*b"expa");
const C1: u32 = u32::from_le_bytes(*b"nd 3");
const C2: u32 = u32::from_le_bytes(*b"2-by");
const C3: u32 = u32::from_le_bytes(*b"te k");

pub fn select_impl() -> RefillFn {
    if cfg!(target_arch = "x86_64") || cfg!(target_arch = "aarch64") {
        // These targets always have 128 bit SIMD available
        return simd128::fill_buf;
    }
    scalar::fill_buf
}
