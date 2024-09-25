use crate::guts::{C0, C1, C2, C3};
use arrayref::{array_mut_ref, mut_array_refs};
use std::arch::x86_64::{
    __m256i, _mm256_add_epi32, _mm256_set1_epi32, _mm256_setr_epi32, _mm256_slli_epi32,
    _mm256_srli_epi32, _mm256_storeu2_m128i, _mm256_xor_si256,
};

/// # Safety
///
/// Requires AVX2 target feature. No other safety requirements.
#[target_feature(enable = "avx2")]
pub unsafe fn fill_buf(key: &[u32; 8], buf: &mut [u32; 256]) {
    let mut ctr = _mm256_setr_epi32(0, 1, 2, 3, 4, 5, 6, 7);

    for eight_blocks in 0..2 {
        #[rustfmt::skip]
        let mut x = [
            splat(C0),     splat(C1),     splat(C2),     splat(C3),
            splat(key[0]), splat(key[1]), splat(key[2]), splat(key[3]),
            splat(key[4]), splat(key[5]), splat(key[6]), splat(key[7]),
            ctr,           splat(0),      splat(0),      splat(0)
        ];

        const ROUNDS: usize = 8;
        for _ in (0..ROUNDS).step_by(2) {
            quarter_round(&mut x, 0, 4, 8, 12);
            quarter_round(&mut x, 1, 5, 9, 13);
            quarter_round(&mut x, 2, 6, 10, 14);
            quarter_round(&mut x, 3, 7, 11, 15);

            quarter_round(&mut x, 0, 5, 10, 15);
            quarter_round(&mut x, 1, 6, 11, 12);
            quarter_round(&mut x, 2, 7, 8, 13);
            quarter_round(&mut x, 3, 4, 9, 14);
        }

        for i in 4..12 {
            x[i] = _mm256_add_epi32(x[i], splat(key[i - 4]));
        }

        let out: &mut [u32; 128] = array_mut_ref![buf, eight_blocks * 128, 128];
        let (out_lo, out_hi) = mut_array_refs![out, 64, 64];
        for (i, &xi) in x.iter().enumerate() {
            let dest_lo: &mut [u32; 4] = array_mut_ref![out_lo, i * 4, 4];
            let dest_hi: &mut [u32; 4] = array_mut_ref![out_hi, i * 4, 4];
            _mm256_storeu2_m128i(dest_hi.as_mut_ptr().cast(), dest_lo.as_mut_ptr().cast(), xi);
        }

        ctr = _mm256_add_epi32(ctr, splat(8));
    }
}

#[target_feature(enable = "avx2")]
unsafe fn splat(a: u32) -> __m256i {
    _mm256_set1_epi32(a as i32)
}

#[target_feature(enable = "avx2")]
#[inline]
unsafe fn quarter_round(x: &mut [__m256i; 16], a: usize, b: usize, c: usize, d: usize) {
    // a += b; d ^= a; d = rotl(d, 16);
    x[a] = _mm256_add_epi32(x[a], x[b]);
    x[d] = _mm256_xor_si256(x[d], x[a]);
    x[d] = rotl16(x[d]);

    // c += d; b ^= c; b = rotl(b, 12);
    x[c] = _mm256_add_epi32(x[c], x[d]);
    x[b] = _mm256_xor_si256(x[b], x[c]);
    x[b] = rotl12(x[b]);

    // a += b; d ^= a; d = rotl(d, 8);
    x[a] = _mm256_add_epi32(x[a], x[b]);
    x[d] = _mm256_xor_si256(x[d], x[a]);
    x[d] = rotl8(x[d]);

    // c += d; b ^= c; b = rotl(b, 7);
    x[c] = _mm256_add_epi32(x[c], x[d]);
    x[b] = _mm256_xor_si256(x[b], x[c]);
    x[b] = rotl7(x[b]);
}

#[target_feature(enable = "avx2")]
unsafe fn rotl16(a: __m256i) -> __m256i {
    _mm256_xor_si256(
        _mm256_slli_epi32::<16>(a),
        _mm256_srli_epi32::<{ 32 - 16 }>(a),
    )
}

#[target_feature(enable = "avx2")]
unsafe fn rotl12(a: __m256i) -> __m256i {
    _mm256_xor_si256(
        _mm256_slli_epi32::<12>(a),
        _mm256_srli_epi32::<{ 32 - 12 }>(a),
    )
}

#[target_feature(enable = "avx2")]
unsafe fn rotl8(a: __m256i) -> __m256i {
    _mm256_xor_si256(
        _mm256_slli_epi32::<8>(a),
        _mm256_srli_epi32::<{ 32 - 8 }>(a),
    )
}

#[target_feature(enable = "avx2")]
unsafe fn rotl7(a: __m256i) -> __m256i {
    _mm256_xor_si256(
        _mm256_slli_epi32::<7>(a),
        _mm256_srli_epi32::<{ 32 - 7 }>(a),
    )
}
