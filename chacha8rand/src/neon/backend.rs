use core::arch::aarch64::uint32x4_t;

use arrayref::array_mut_ref;

use crate::{
    neon::safe_arch::{
        add_u32, reinterpret_u32x4_as_u8x16, reinterpret_u8x16_as_u32x4, shift_left_u32, splat,
        store_u8x16, tbl_u8x16, u32x4_from_elems, xor,
    },
    Backend, Buffer, C0, C1, C2, C3,
};

use super::safe_arch::{
    reinterpret_u16x8_as_u32x4, reinterpret_u32x4_as_u16x8, rev32_u16, shift_right_insert_u32,
    u8x16_from_elems,
};

pub fn detect() -> Option<Backend> {
    Some(Backend::new(fill_buf))
}

pub fn fill_buf(key: &[u32; 8], buf: &mut Buffer) {
    let buf = &mut buf.bytes;
    let mut ctr = u32x4_from_elems([0, 1, 2, 3]);
    for group in 0..4 {
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
            x[i] = add_u32(x[i], splat(key[i - 4]));
        }

        let group_buf = array_mut_ref![buf, group * 256, 256];
        for (i, &xi) in x.iter().enumerate() {
            store_u8x16(
                reinterpret_u32x4_as_u8x16(xi),
                array_mut_ref![group_buf, 16 * i, 16],
            );
        }

        ctr = add_u32(ctr, splat(4));
    }
}

#[inline(always)]
fn quarter_round(x: &mut [uint32x4_t; 16], a: usize, b: usize, c: usize, d: usize) {
    // a += b; d ^= a; d = rotl(d, 16);
    x[a] = add_u32(x[a], x[b]);
    x[d] = xor(x[d], x[a]);
    x[d] = rotl16(x[d]);

    // c += d; b ^= c; b = rotl(b, 12);
    x[c] = add_u32(x[c], x[d]);
    x[b] = xor(x[b], x[c]);
    x[b] = rotl::<12, 20>(x[b]);

    // a += b; d ^= a; d = rotl(d, 8);
    x[a] = add_u32(x[a], x[b]);
    x[d] = xor(x[d], x[a]);
    x[d] = rotl8(x[d]);

    // c += d; b ^= c; b = rotl(b, 7);
    x[c] = add_u32(x[c], x[d]);
    x[b] = xor(x[b], x[c]);
    x[b] = rotl::<7, 25>(x[b]);
}

#[inline(always)]
fn rotl16(x: uint32x4_t) -> uint32x4_t {
    // There's a dedicated instruction for swapping the 16-bit halves of every 32-bit lane, which
    // is faster than generic rotate-left-by-k sequences but gives the same result. For example:
    const {
        assert!(0x1234_5678u32.rotate_left(16) == 0x5678_1234);
    }
    reinterpret_u16x8_as_u32x4(rev32_u16(reinterpret_u32x4_as_u16x8(x)))
}

#[inline(always)]
fn rotl8(x: uint32x4_t) -> uint32x4_t {
    // This rotation can be implemented as a byte shuffle with VTBL, which has better throughput and
    // latency than a shift -> shift-insert chain on every core I've checked. At least if loading
    // the index into a register is amortized over several quarter rounds, which it should be, since
    // we expect everything to be inlined into a loop body doing eight quarter-rounds per iteration.
    #[rustfmt::skip]
    static ROTL8_TBL_IDX: [u8; 16] = [
        3, 0, 1, 2,
        7, 4, 5, 6,
        11, 8, 9, 10,
        15, 12, 13, 14
    ];
    let idx = u8x16_from_elems(ROTL8_TBL_IDX);
    reinterpret_u8x16_as_u32x4(tbl_u8x16(reinterpret_u32x4_as_u8x16(x), idx))
}

#[inline(always)]
fn rotl<const SH_LEFT: i32, const SH_RIGHT: i32>(x: uint32x4_t) -> uint32x4_t {
    const {
        assert!(SH_RIGHT == (32 - SH_LEFT));
    }
    // The other rotates (by 12 and by 7) don't seem to have faster implementations than a pair of
    // shift and shift-insert.
    shift_right_insert_u32::<SH_RIGHT>(shift_left_u32::<SH_LEFT>(x), x)
}
