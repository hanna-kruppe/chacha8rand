use core::arch::wasm32::{u32x4_splat, v128, v128_store};

// This trivial wrapper is needed because the function from core::arch has a `#[target_feature]`
// annotation, which prevents it from implementing the `Fn` traits, which we need to pass it as
// callback into a helper function.
#[inline(always)]
pub fn splat(x: u32) -> v128 {
    u32x4_splat(x)
}

pub fn store_as_u8x16(x: v128, dest: &mut [u8; 16]) {
    // SAFETY: stores 16 bytes through the pointer (without alignment requirement), which is OK
    // because we pass a `&mut [u8; 16]`.
    unsafe {
        v128_store(dest.as_mut_ptr().cast(), x);
    }
}
