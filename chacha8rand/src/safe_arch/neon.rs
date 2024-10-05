use core::arch::aarch64::{
    uint32x4_t, vaddq_u32, vdupq_n_u32, veorq_u32, vld1q_u32, vshlq_n_u32, vshrq_n_u32, vst1q_u32,
};

// This is redundant with the cfg() this module is gated on, but since we're going to be calling
// core::arch intrinsics it doesn't hurt to double-check that we actually have the necessary target
// feature.
const _: () = assert!(cfg!(target_arch = "aarch64") && cfg!(target_feature = "neon"));

pub fn splat(x: u32) -> uint32x4_t {
    // SAFETY: requires the neon target feature, which was detected via cfg.
    unsafe { vdupq_n_u32(x) }
}

pub fn from_elems(elems: [u32; 4]) -> uint32x4_t {
    // SAFETY: (1) Requires the neon target feature, which was detected via cfg. (2) Loads 128 bits
    // from the pointer, which is OK since we pass the address of a `[u32; 4]`.
    unsafe { vld1q_u32(elems.as_ptr()) }
}

pub fn add_u32(x: uint32x4_t, y: uint32x4_t) -> uint32x4_t {
    // SAFETY: requires the neon target feature, which was detected via cfg.
    unsafe { vaddq_u32(x, y) }
}

pub fn xor(x: uint32x4_t, y: uint32x4_t) -> uint32x4_t {
    // SAFETY: requires the neon target feature, which was detected via cfg.
    unsafe { veorq_u32(x, y) }
}

pub fn shift_left_u32<const N: i32>(x: uint32x4_t) -> uint32x4_t {
    // SAFETY: requires the neon target feature, which was detected via cfg.
    unsafe { vshlq_n_u32::<N>(x) }
}

pub fn shift_right_u32<const N: i32>(x: uint32x4_t) -> uint32x4_t {
    // SAFETY: requires the neon target feature, which was detected via cfg.
    unsafe { vshrq_n_u32::<N>(x) }
}

pub fn store_u32x4(x: uint32x4_t, dest: &mut [u32; 4]) {
    // SAFETY: (1) Requires the neon target feature, which was detected by cfg. (2) Stores 128 bits
    // through the pointer, which is OK because it's a mutable reference to `[u32; 4]`.
    unsafe {
        vst1q_u32(dest.as_mut_ptr(), x);
    }
}
