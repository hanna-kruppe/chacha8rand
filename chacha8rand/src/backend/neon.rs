use core::arch::aarch64::{
    uint8x16_t, uint32x4_t, vaddq_u32, vcombine_u8, vcombine_u32, vcreate_u8, vcreate_u32,
    vdupq_n_u32, veorq_u32, vqtbl1q_u8, vreinterpretq_u8_u32, vreinterpretq_u16_u32,
    vreinterpretq_u32_u8, vreinterpretq_u32_u16, vrev32q_u16, vshlq_n_u32, vsriq_n_u32, vst1q_u8,
};

use crate::{
    Backend, Buffer,
    array_ref::array_chunks_mut,
    backend::{eight_rounds, init_state},
};

pub(crate) fn detect() -> Option<Backend> {
    #[cfg(feature = "std")]
    let has_neon = std::arch::is_aarch64_feature_detected!("neon");
    #[cfg(not(feature = "std"))]
    let has_neon = cfg!(target_feature = "neon");
    if has_neon {
        // SAFETY: `fill_buf` is safe to call because neon is available.
        Some(unsafe { Backend::new_unchecked(fill_buf) })
    } else {
        None
    }
}

#[target_feature(enable = "neon")]
fn fill_buf(key: &[u32; 8], buf: &mut Buffer) {
    let splat = |x| vdupq_n_u32(x);

    let buf = &mut buf.bytes;
    let mut ctr = vcombine_u32(vcreate_u32(1u64 << 32), vcreate_u32(2u64 | (3u64 << 32)));
    for group in array_chunks_mut::<256, 1024>(buf) {
        let mut x = init_state(ctr, key, splat);

        eight_rounds(&mut x, |abcd| quarter_round(abcd));

        for i in 4..12 {
            x[i] = vaddq_u32(x[i], splat(key[i - 4]));
        }

        array_chunks_mut::<16, 256>(group)
            .zip(x)
            .for_each(|(dest, xi)| {
                store_u8x16(vreinterpretq_u8_u32(xi), dest);
            });

        ctr = vaddq_u32(ctr, splat(4));
    }
}

#[inline]
#[target_feature(enable = "neon")]
fn quarter_round([mut a, mut b, mut c, mut d]: [uint32x4_t; 4]) -> [uint32x4_t; 4] {
    a = vaddq_u32(a, b);
    d = veorq_u32(d, a);
    d = rotl16(d);

    c = vaddq_u32(c, d);
    b = veorq_u32(b, c);
    b = rotl::<12, 20>(b);

    a = vaddq_u32(a, b);
    d = veorq_u32(d, a);
    d = rotl8(d);

    c = vaddq_u32(c, d);
    b = veorq_u32(b, c);
    b = rotl::<7, 25>(b);

    [a, b, c, d]
}

#[inline]
#[target_feature(enable = "neon")]
fn rotl16(x: uint32x4_t) -> uint32x4_t {
    // There's a dedicated instruction for swapping the 16-bit halves of every 32-bit lane, which
    // is faster than generic rotate-left-by-k sequences but gives the same result. For example:
    const {
        assert!(0x1234_5678u32.rotate_left(16) == 0x5678_1234);
    }
    vreinterpretq_u32_u16(vrev32q_u16(vreinterpretq_u16_u32(x)))
}

#[inline]
#[target_feature(enable = "neon")]
fn rotl8(x: uint32x4_t) -> uint32x4_t {
    // This rotation can be implemented as a byte shuffle with VTBL, which has better throughput and
    // latency than a shift -> shift-insert chain on every core I've checked. At least if loading
    // the index into a register is amortized over several quarter rounds, which it should be, since
    // we expect everything to be inlined into a loop body doing eight quarter-rounds per iteration.
    #[rustfmt::skip]
    static ROTL8_TBL_IDX: [u8; 16] = [
         3,  0,  1,  2,
         7,  4,  5,  6,
        11,  8,  9, 10,
        15, 12, 13, 14,
    ];
    const IDX_WORD_LO: u64 = u128::from_le_bytes(ROTL8_TBL_IDX) as u64;
    const IDX_WORD_HI: u64 = (u128::from_le_bytes(ROTL8_TBL_IDX) >> 64) as u64;
    let idx = vcombine_u8(vcreate_u8(IDX_WORD_LO), vcreate_u8(IDX_WORD_HI));
    vreinterpretq_u32_u8(vqtbl1q_u8(vreinterpretq_u8_u32(x), idx))
}

#[inline]
#[target_feature(enable = "neon")]
fn rotl<const SH_LEFT: i32, const SH_RIGHT: i32>(x: uint32x4_t) -> uint32x4_t {
    const {
        assert!(SH_RIGHT == (32 - SH_LEFT));
    }
    // The other rotates (by 12 and by 7) don't seem to have faster implementations than a pair of
    // shift and shift-insert.
    vsriq_n_u32::<SH_RIGHT>(vshlq_n_u32::<SH_LEFT>(x), x)
}

fn store_u8x16(x: uint8x16_t, dest: &mut [u8; 16]) {
    // SAFETY: Stores 128 bits through the pointer (no alignment requirement), which is OK because
    // it's a mutable reference to `[u8; 16]`.
    unsafe {
        vst1q_u8(dest.as_mut_ptr(), x);
    }
}
