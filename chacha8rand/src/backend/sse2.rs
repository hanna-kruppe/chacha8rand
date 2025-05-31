#[cfg(target_arch = "x86")]
use core::arch::x86;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64 as x86;

use x86::{
    __m128i, _mm_add_epi32, _mm_set1_epi32, _mm_setr_epi32, _mm_slli_epi32, _mm_srli_epi32,
    _mm_storeu_si128, _mm_xor_si128,
};

use crate::{
    Backend, Buffer,
    array_ref::array_chunks_mut,
    common_guts::{eight_rounds, init_state},
};

pub(crate) fn detect() -> Option<Backend> {
    #[cfg(feature = "std")]
    let has_sse2 = std::arch::is_x86_feature_detected!("sse2");
    #[cfg(not(feature = "std"))]
    let has_sse2 = cfg!(target_feature = "sse2");
    if has_sse2 {
        // SAFETY: `fill_buf` is safe to call because SSE2 is available.
        Some(unsafe { Backend::new_unchecked(fill_buf) })
    } else {
        None
    }
}

#[target_feature(enable = "sse2")]
fn fill_buf(key: &[u32; 8], buf: &mut Buffer) {
    let splat = |x: u32| _mm_set1_epi32(x.cast_signed());

    let buf = &mut buf.bytes;
    let mut ctr = _mm_setr_epi32(0, 1, 2, 3);
    for group in array_chunks_mut::<256, 1024>(buf) {
        let mut x = init_state(ctr, key, splat);

        eight_rounds(&mut x, |abcd| quarter_round(abcd));

        for i in 4..12 {
            x[i] = _mm_add_epi32(x[i], splat(key[i - 4]));
        }

        array_chunks_mut::<16, 256>(group)
            .zip(x)
            .for_each(|(dest, xi)| {
                store_unaligned(dest, xi);
            });

        ctr = _mm_add_epi32(ctr, splat(4));
    }
}

#[target_feature(enable = "sse2")]
fn store_unaligned(dest: &mut [u8; 16], xi: __m128i) {
    // TODO: can we get the same codegen without any unsafe? Maybe repeated _mm_cvtsi128_si32 +
    // _mm_shuffle_epi32 can get optimized into an unaligned store.
    let mem_addr: *mut __m128i = dest.as_mut_ptr().cast();
    // SAFETY: Stores 128 bits through the pointer, which is OK because it's a mutable reference to
    // `[u8; 16]`. There is no alignment requirement.
    unsafe {
        _mm_storeu_si128(mem_addr, xi);
    }
}

#[inline]
#[target_feature(enable = "sse2")]
fn quarter_round([mut a, mut b, mut c, mut d]: [__m128i; 4]) -> [__m128i; 4] {
    a = _mm_add_epi32(a, b);
    d = _mm_xor_si128(d, a);
    d = rotl::<16, 16>(d);

    c = _mm_add_epi32(c, d);
    b = _mm_xor_si128(b, c);
    b = rotl::<12, 20>(b);

    a = _mm_add_epi32(a, b);
    d = _mm_xor_si128(d, a);
    d = rotl::<8, 24>(d);

    c = _mm_add_epi32(c, d);
    b = _mm_xor_si128(b, c);
    b = rotl::<7, 25>(b);

    [a, b, c, d]
}

#[inline]
#[target_feature(enable = "sse2")]
fn rotl<const SH_LEFT: i32, const SH_RIGHT: i32>(x: __m128i) -> __m128i {
    const {
        assert!(SH_RIGHT == (32 - SH_LEFT));
    }
    _mm_xor_si128(_mm_slli_epi32::<SH_LEFT>(x), _mm_srli_epi32::<SH_RIGHT>(x))
}
