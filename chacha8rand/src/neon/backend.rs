use core::arch::aarch64::uint32x4_t;

use arrayref::array_mut_ref;

use crate::{
    neon::safe_arch::{
        add_u32, from_elems, shift_left_u32, shift_right_u32, splat, store_u32x4, xor,
    },
    Backend, Buffer, C0, C1, C2, C3,
};

pub fn detect() -> Option<Backend> {
    Some(Backend::new(fill_buf))
}

pub fn fill_buf(key: &[u32; 8], buf: &mut Buffer) {
    let buf = &mut buf.words;
    let mut ctr = from_elems([0, 1, 2, 3]);
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

        let group_buf = array_mut_ref![buf, group * 64, 64];
        for (i, &xi) in x.iter().enumerate() {
            store_u32x4(xi, array_mut_ref![group_buf, 4 * i, 4]);
        }

        ctr = add_u32(ctr, splat(4));
    }
}

#[inline(always)]
fn quarter_round(x: &mut [uint32x4_t; 16], a: usize, b: usize, c: usize, d: usize) {
    // a += b; d ^= a; d = rotl(d, 16);
    x[a] = add_u32(x[a], x[b]);
    x[d] = xor(x[d], x[a]);
    x[d] = rotl::<16, 16>(x[d]);

    // c += d; b ^= c; b = rotl(b, 12);
    x[c] = add_u32(x[c], x[d]);
    x[b] = xor(x[b], x[c]);
    x[b] = rotl::<12, 20>(x[b]);

    // a += b; d ^= a; d = rotl(d, 8);
    x[a] = add_u32(x[a], x[b]);
    x[d] = xor(x[d], x[a]);
    x[d] = rotl::<8, 24>(x[d]);

    // c += d; b ^= c; b = rotl(b, 7);
    x[c] = add_u32(x[c], x[d]);
    x[b] = xor(x[b], x[c]);
    x[b] = rotl::<7, 25>(x[b]);
}

#[inline(always)]
fn rotl<const SH_LEFT: i32, const SH_RIGHT: i32>(x: uint32x4_t) -> uint32x4_t {
    const {
        assert!(SH_RIGHT == (32 - SH_LEFT));
    }
    xor(shift_left_u32::<SH_LEFT>(x), shift_right_u32::<SH_RIGHT>(x))
}
