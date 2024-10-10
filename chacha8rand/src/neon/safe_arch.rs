use core::arch::aarch64::{
    uint16x8_t, uint32x4_t, uint8x16_t, vaddq_u32, vdupq_n_u32, veorq_u32, vld1q_u32, vld1q_u8,
    vqtbl1q_u8, vreinterpretq_u16_u32, vreinterpretq_u32_u16, vreinterpretq_u32_u8,
    vreinterpretq_u8_u32, vrev32q_u16, vshlq_n_u32, vsriq_n_u32, vst1q_u8,
};

// This is redundant with the cfg() this module is gated on, but since we're going to be calling
// core::arch intrinsics it doesn't hurt to double-check that we actually have the necessary target
// feature.
const _: () = assert!(cfg!(target_arch = "aarch64") && cfg!(target_feature = "neon"));

pub fn splat(x: u32) -> uint32x4_t {
    // SAFETY: requires the neon target feature, which was detected via cfg.
    unsafe { vdupq_n_u32(x) }
}

pub fn u32x4_from_elems(elems: [u32; 4]) -> uint32x4_t {
    // SAFETY: (1) Requires the neon target feature, which was detected via cfg. (2) Loads 128 bits
    // from the pointer, which is OK since we pass the address of a `[u32; 4]`.
    unsafe { vld1q_u32(elems.as_ptr()) }
}

pub fn u8x16_from_elems(elems: [u8; 16]) -> uint8x16_t {
    // SAFETY: (1) Requires the neon target feature, which was detected via cfg. (2) Loads 128 bits
    // from the pointer, which is OK since we pass the address of a `[u8; 16]`.
    unsafe { vld1q_u8(elems.as_ptr()) }
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

pub fn shift_right_insert_u32<const N: i32>(x: uint32x4_t, y: uint32x4_t) -> uint32x4_t {
    // SAFETY: requires the neon target feature, which was detected via cfg.
    unsafe { vsriq_n_u32::<N>(x, y) }
}

pub fn reinterpret_u32x4_as_u16x8(x: uint32x4_t) -> uint16x8_t {
    // SAFETY: requires the neon target feature, which was detected via cfg.
    unsafe { vreinterpretq_u16_u32(x) }
}

pub fn reinterpret_u16x8_as_u32x4(x: uint16x8_t) -> uint32x4_t {
    // SAFETY: requires the neon target feature, which was detected via cfg.
    unsafe { vreinterpretq_u32_u16(x) }
}

pub fn reinterpret_u32x4_as_u8x16(x: uint32x4_t) -> uint8x16_t {
    // SAFETY: requires the neon target feature, which was detected via cfg.
    unsafe { vreinterpretq_u8_u32(x) }
}

pub fn reinterpret_u8x16_as_u32x4(x: uint8x16_t) -> uint32x4_t {
    // SAFETY: requires the neon target feature, which was detected via cfg.
    unsafe { vreinterpretq_u32_u8(x) }
}

pub fn rev32_u16(x: uint16x8_t) -> uint16x8_t {
    // SAFETY: requires the neon target feature, which was detected via cfg.
    unsafe { vrev32q_u16(x) }
}

pub fn tbl_u8x16(t: uint8x16_t, idx: uint8x16_t) -> uint8x16_t {
    unsafe { vqtbl1q_u8(t, idx) }
}

pub fn store_u8x16(x: uint8x16_t, dest: &mut [u8; 16]) {
    // SAFETY: (1) Requires the neon target feature, which was detected by cfg. (2) Stores 128 bits
    // through the pointer, which is OK because it's a mutable reference to `[u8; 16]`.
    unsafe {
        vst1q_u8(dest.as_mut_ptr(), x);
    }
}
