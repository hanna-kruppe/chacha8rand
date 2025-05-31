#[cfg(target_arch = "x86")]
use core::arch::x86;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64 as x86;

use x86::{
    __m128i, __m256i, _mm256_add_epi32, _mm256_set1_epi32, _mm256_setr_epi32, _mm256_slli_epi32,
    _mm256_srli_epi32, _mm256_storeu2_m128i, _mm256_xor_si256,
};

use crate::{
    Backend, Buffer,
    array_ref::{array_chunks_mut, slice_array_mut},
    backend::{eight_rounds, init_state},
};

pub(crate) fn detect() -> Option<Backend> {
    #[cfg(feature = "std")]
    let has_avx2 = std::is_x86_feature_detected!("avx2");
    #[cfg(not(feature = "std"))]
    let has_avx2 = cfg!(target_feature = "avx2");
    if has_avx2 {
        // SAFETY: `fill_buf` is only unsafe because it enables the AVX2 `target_feature`, and we've
        // ensured that AVX2 is available, so it's now effectively a safe function.
        unsafe { Some(Backend::new_unchecked(fill_buf)) }
    } else {
        None
    }
}

#[target_feature(enable = "avx2")]
fn fill_buf(key: &[u32; 8], buf: &mut Buffer) {
    let buf = &mut buf.bytes;
    let mut ctr = _mm256_setr_epi32(0, 1, 2, 3, 4, 5, 6, 7);
    let splat = |x: u32| _mm256_set1_epi32(x.cast_signed());

    for eight_blocks in array_chunks_mut::<512, 1024>(buf) {
        let mut x = init_state(ctr, key, splat);

        eight_rounds(&mut x, |abcd| quarter_round(abcd));

        for i in 4..12 {
            x[i] = _mm256_add_epi32(x[i], splat(key[i - 4]));
        }

        let (out_lo, out_hi) = eight_blocks.split_at_mut(256);
        let out_lo: &mut [u8; 256] = out_lo.try_into().unwrap();
        let out_hi: &mut [u8; 256] = out_hi.try_into().unwrap();
        for (i, &xi) in x.iter().enumerate() {
            let dest_lo: &mut [u8; 16] = slice_array_mut::<16>(out_lo, i * 16);
            let dest_hi: &mut [u8; 16] = slice_array_mut::<16>(out_hi, i * 16);
            let hiaddr: *mut __m128i = dest_hi.as_mut_ptr().cast();
            let loaddr: *mut __m128i = dest_lo.as_mut_ptr().cast();
            // SAFETY: this stores 128 bits to each of the two addresses. (There are no alignment
            // requirements.) Writing to both destinations is OK because both pointers are derived
            // from distinct `&mut [u8; 16]`, i.e., we're allowed to write 128 bits to both of those
            // locations.
            unsafe {
                _mm256_storeu2_m128i(hiaddr, loaddr, xi);
            };
        }

        ctr = _mm256_add_epi32(ctr, splat(8));
    }
}

#[inline]
#[target_feature(enable = "avx2")]
fn quarter_round([mut a, mut b, mut c, mut d]: [__m256i; 4]) -> [__m256i; 4] {
    a = _mm256_add_epi32(a, b);
    d = _mm256_xor_si256(d, a);
    d = rotl::<16, 16>(d);

    c = _mm256_add_epi32(c, d);
    b = _mm256_xor_si256(b, c);
    b = rotl::<12, 20>(b);

    a = _mm256_add_epi32(a, b);
    d = _mm256_xor_si256(d, a);
    d = rotl::<8, 24>(d);

    c = _mm256_add_epi32(c, d);
    b = _mm256_xor_si256(b, c);
    b = rotl::<7, 25>(b);

    [a, b, c, d]
}

#[inline]
#[target_feature(enable = "avx2")]
fn rotl<const SH_LEFT: i32, const SH_RIGHT: i32>(x: __m256i) -> __m256i {
    // Note: some of these rotations can be implemented as shuffles, but LLVM manages to figure that
    // out by itself, so there's no need to complicate the code.
    const {
        assert!(SH_RIGHT == (32 - SH_LEFT));
    }
    _mm256_xor_si256(
        _mm256_slli_epi32::<SH_LEFT>(x),
        _mm256_srli_epi32::<SH_RIGHT>(x),
    )
}
