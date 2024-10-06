use crate::{
    avx2::safe_arch::{Avx2, __m256i},
    Backend, Buffer, C0, C1, C2, C3,
};
use arrayref::{array_mut_ref, mut_array_refs};

// See sibling module for rationale
extern crate std;

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

    let buf = &mut buf.words;
    let mut ctr = avx2.elems([0, 1, 2, 3, 4, 5, 6, 7]);
    let splat = |x| avx2.splat(x);

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
            quarter_round(avx2, &mut x, 0, 4, 8, 12);
            quarter_round(avx2, &mut x, 1, 5, 9, 13);
            quarter_round(avx2, &mut x, 2, 6, 10, 14);
            quarter_round(avx2, &mut x, 3, 7, 11, 15);

            quarter_round(avx2, &mut x, 0, 5, 10, 15);
            quarter_round(avx2, &mut x, 1, 6, 11, 12);
            quarter_round(avx2, &mut x, 2, 7, 8, 13);
            quarter_round(avx2, &mut x, 3, 4, 9, 14);
        }

        for i in 4..12 {
            x[i] = avx2.add_u32(x[i], splat(key[i - 4]));
        }

        let out: &mut [u32; 128] = array_mut_ref![buf, eight_blocks * 128, 128];
        let (out_lo, out_hi) = mut_array_refs![out, 64, 64];
        for (i, &xi) in x.iter().enumerate() {
            let dest_lo: &mut [u32; 4] = array_mut_ref![out_lo, i * 4, 4];
            let dest_hi: &mut [u32; 4] = array_mut_ref![out_hi, i * 4, 4];
            avx2.storeu2(xi, dest_hi, dest_lo);
        }

        ctr = avx2.add_u32(ctr, splat(8));
    }
}

#[inline(always)]
fn quarter_round(avx2: Avx2, x: &mut [__m256i; 16], a: usize, b: usize, c: usize, d: usize) {
    // a += b; d ^= a; d = rotl(d, 16);
    x[a] = avx2.add_u32(x[a], x[b]);
    x[d] = avx2.xor(x[d], x[a]);
    x[d] = rotl::<16, 16>(avx2, x[d]);

    // c += d; b ^= c; b = rotl(b, 12);
    x[c] = avx2.add_u32(x[c], x[d]);
    x[b] = avx2.xor(x[b], x[c]);
    x[b] = rotl::<12, 20>(avx2, x[b]);

    // a += b; d ^= a; d = rotl(d, 8);
    x[a] = avx2.add_u32(x[a], x[b]);
    x[d] = avx2.xor(x[d], x[a]);
    x[d] = rotl::<8, 24>(avx2, x[d]);

    // c += d; b ^= c; b = rotl(b, 7);
    x[c] = avx2.add_u32(x[c], x[d]);
    x[b] = avx2.xor(x[b], x[c]);
    x[b] = rotl::<7, 25>(avx2, x[b]);
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
