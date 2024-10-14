use crate::{
    avx2::safe_arch::{Avx2, __m256i},
    common_guts::{eight_rounds, init_state},
    Backend, Buffer,
};
use arrayref::{array_mut_ref, mut_array_refs};

pub(crate) fn detect() -> Option<Backend> {
    if std::is_x86_feature_detected!("avx2") {
        // SAFETY: `fill_buf` is only unsafe because it enables the AVX2 `target_feature`, and we've
        // ensured that AVX2 is available, so it's now effectively a safe function.
        unsafe { Some(Backend::new_unchecked(fill_buf)) }
    } else {
        None
    }
}

/// # Safety
///
/// Requires AVX2 target feature. No other safety requirements.
#[target_feature(enable = "avx2")]
pub unsafe fn fill_buf(key: &[u32; 8], buf: &mut Buffer) {
    // Since we're already inside a function with `target_feature(enable = "avx2)`, the `expect` is
    // too late to prevent UB. But there is still a chance that it panics if that UB is triggered,
    // and the check is basically free compared to the work we're doing below, so it doesn't hurt to
    // use `expect` here.
    let avx2 = Avx2::new().expect("AVX2 must be available if this backend is invoked");

    let buf = &mut buf.bytes;
    let mut ctr = avx2.elems([0, 1, 2, 3, 4, 5, 6, 7]);
    let splat = |x| avx2.splat(x);

    for eight_blocks in 0..2 {
        let mut x = init_state(ctr, key, splat);

        eight_rounds(
            &mut x,
            #[inline(always)]
            |abcd| quarter_round(avx2, abcd),
        );

        for i in 4..12 {
            x[i] = avx2.add_u32(x[i], splat(key[i - 4]));
        }

        let out: &mut [u8; 512] = array_mut_ref![buf, eight_blocks * 512, 512];
        let (out_lo, out_hi) = mut_array_refs![out, 256, 256];
        for (i, &xi) in x.iter().enumerate() {
            let dest_lo: &mut [u8; 16] = array_mut_ref![out_lo, i * 16, 16];
            let dest_hi: &mut [u8; 16] = array_mut_ref![out_hi, i * 16, 16];
            avx2.storeu2(xi, dest_hi, dest_lo);
        }

        ctr = avx2.add_u32(ctr, splat(8));
    }
}

#[inline(always)]
fn quarter_round(avx2: Avx2, [mut a, mut b, mut c, mut d]: [__m256i; 4]) -> [__m256i; 4] {
    a = avx2.add_u32(a, b);
    d = avx2.xor(d, a);
    d = rotl::<16, 16>(avx2, d);

    c = avx2.add_u32(c, d);
    b = avx2.xor(b, c);
    b = rotl::<12, 20>(avx2, b);

    a = avx2.add_u32(a, b);
    d = avx2.xor(d, a);
    d = rotl::<8, 24>(avx2, d);

    c = avx2.add_u32(c, d);
    b = avx2.xor(b, c);
    b = rotl::<7, 25>(avx2, b);

    [a, b, c, d]
}

#[inline(always)]
fn rotl<const SH_LEFT: i32, const SH_RIGHT: i32>(avx2: Avx2, x: __m256i) -> __m256i {
    // Note: some of these rotations can be implemented as shuffles, but LLVM manages to figure that
    // out by itself, so there's no need to complicate the code.
    const {
        assert!(SH_RIGHT == (32 - SH_LEFT));
    }
    avx2.xor(
        avx2.shift_left_u32::<SH_LEFT>(x),
        avx2.shift_right_u32::<SH_RIGHT>(x),
    )
}
