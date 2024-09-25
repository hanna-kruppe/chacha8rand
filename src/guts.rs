#[cfg(target_arch = "x86_64")]
pub mod avx2;
pub mod scalar;
pub mod simd128;

// The constant words in the first row of the initial state
const C0: u32 = u32::from_le_bytes(*b"expa");
const C1: u32 = u32::from_le_bytes(*b"nd 3");
const C2: u32 = u32::from_le_bytes(*b"2-by");
const C3: u32 = u32::from_le_bytes(*b"te k");
