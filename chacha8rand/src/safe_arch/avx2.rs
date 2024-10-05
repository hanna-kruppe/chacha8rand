use std::arch::x86_64::{
    __m128i, __m256i, _mm256_add_epi32, _mm256_set1_epi32, _mm256_setr_epi32, _mm256_slli_epi32,
    _mm256_srli_epi32, _mm256_storeu2_m128i, _mm256_xor_si256,
};

pub(crate) use detect::Avx2;

mod detect {
    // Safety invariant: can only be constructed if AVX2 is available.
    #[derive(Clone, Copy)]
    pub(crate) struct Avx2 {
        _feature_detected: (),
    }

    impl Avx2 {
        pub(crate) fn new() -> Option<Self> {
            if std::is_x86_feature_detected!("avx2") {
                Some(Self {
                    _feature_detected: (),
                })
            } else {
                None
            }
        }
    }
}

impl Avx2 {
    #[inline(always)]
    pub(crate) fn elems(self, elems: [u32; 8]) -> __m256i {
        let [e0, e1, e2, e3, e4, e5, e6, e7] = elems.map(|e| e as i32);
        // SAFETY: only needs AVX2, `self` proves that we have AVX2.
        unsafe { _mm256_setr_epi32(e0, e1, e2, e3, e4, e5, e6, e7) }
    }

    #[inline(always)]
    pub(crate) fn splat(self, x: u32) -> __m256i {
        // SAFETY: only needs AVX2, `self` proves that we have AVX2.
        unsafe { _mm256_set1_epi32(x as i32) }
    }

    #[inline(always)]
    pub(crate) fn add_u32(self, a: __m256i, b: __m256i) -> __m256i {
        // SAFETY: only needs AVX2, `self` proves that we have AVX2.
        unsafe { _mm256_add_epi32(a, b) }
    }

    #[inline(always)]
    pub(crate) fn xor(self, a: __m256i, b: __m256i) -> __m256i {
        // SAFETY: only needs AVX2, `self` proves that we have AVX2.
        unsafe { _mm256_xor_si256(a, b) }
    }

    #[inline(always)]
    pub(crate) fn shift_left_u32<const IMM8: i32>(self, a: __m256i) -> __m256i {
        // SAFETY: only needs AVX2, `self` proves that we have AVX2.
        unsafe { _mm256_slli_epi32::<IMM8>(a) }
    }

    #[inline(always)]
    pub(crate) fn shift_right_u32<const IMM8: i32>(self, a: __m256i) -> __m256i {
        // SAFETY: only needs AVX2, `self` proves that we have AVX2.
        unsafe { _mm256_srli_epi32::<IMM8>(a) }
    }

    #[inline(always)]
    pub(crate) fn storeu2(self, vec: __m256i, dest_hi: &mut [u32; 4], dest_lo: &mut [u32; 4]) {
        let hiaddr: *mut __m128i = dest_hi.as_mut_ptr().cast();
        let loaddr: *mut __m128i = dest_lo.as_mut_ptr().cast();
        // SAFETY: this intrinsic requires AVX2 and stores 128 bits to each of the two addresses.
        // (There are no alignment requirements.) `self` proves we have AVX2. Writing to both
        // destinations is OK because both pointers are derived from distinct `&mut [u32; 4]`, i.e.,
        // we're allowed to write 128 bits to both of those locations.
        unsafe {
            _mm256_storeu2_m128i(hiaddr, loaddr, vec);
        }
    }
}
