#[cfg(target_arch = "x86")]
use core::arch::x86 as arch;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64 as arch;

pub use arch::__m128i;
use arch::{
    _mm_add_epi32, _mm_set1_epi32, _mm_setr_epi32, _mm_slli_epi32, _mm_srli_epi32,
    _mm_storeu_si128, _mm_xor_si128,
};

// This is redundant with the cfg() this module is gated on, but since we're going to be calling
// core::arch intrinsics it doesn't hurt to double-check that we actually have the necessary target
// feature.
const _: () = assert!(
    cfg!(any(target_arch = "x86_64", target_arch = "x86")) && cfg!(target_feature = "sse2")
);

pub fn from_elems(elems: [u32; 4]) -> __m128i {
    let [e0, e1, e2, e3] = elems.map(|x| x as i32);
    // SAFETY: requires the sse2 target feature, which was detected via cfg.
    unsafe { _mm_setr_epi32(e0, e1, e2, e3) }
}

pub fn splat(x: u32) -> __m128i {
    // SAFETY: requires the sse2 target feature, which was detected via cfg.
    unsafe { _mm_set1_epi32(x as i32) }
}

pub fn add_u32(x: __m128i, y: __m128i) -> __m128i {
    // SAFETY: requires the sse2 target feature, which was detected via cfg.
    unsafe { _mm_add_epi32(x, y) }
}

pub fn xor(x: __m128i, y: __m128i) -> __m128i {
    // SAFETY: requires the sse2 target feature, which was detected via cfg.
    unsafe { _mm_xor_si128(x, y) }
}

pub fn shift_left_u32<const IMM8: i32>(x: __m128i) -> __m128i {
    // SAFETY: requires the sse2 target feature, which was detected via cfg.
    unsafe { _mm_slli_epi32::<IMM8>(x) }
}

pub fn shift_right_u32<const IMM8: i32>(x: __m128i) -> __m128i {
    // SAFETY: requires the sse2 target feature, which was detected via cfg.
    unsafe { _mm_srli_epi32::<IMM8>(x) }
}

pub fn storeu(x: __m128i, dest: &mut [u8; 16]) {
    // SAFETY: (1) Requires the sse2 target feature, which was detected by cfg. (2) Stores 128 bits
    // through the pointer, which is OK because it's a mutable reference to `[u8; 16]`. There is no
    // alignment requirement.
    let mem_addr: *mut __m128i = dest.as_mut_ptr().cast();
    unsafe {
        _mm_storeu_si128(mem_addr, x);
    }
}
