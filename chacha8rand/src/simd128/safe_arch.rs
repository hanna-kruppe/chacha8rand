use core::arch::wasm32::{v128, v128_store};

pub fn store_as_u8x16(x: v128, dest: &mut [u8; 16]) {
    // SAFETY: stores 16 bytes through the pointer (without alignment requirement), which is OK
    // because we pass a `&mut [u8; 16]`.
    unsafe {
        v128_store(dest.as_mut_ptr().cast(), x);
    }
}
