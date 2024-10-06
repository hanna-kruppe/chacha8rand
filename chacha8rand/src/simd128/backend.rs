use core::arch::wasm32::{
    u32x4, u32x4_add, u32x4_shl, u32x4_shr, u32x4_splat as splat, v128, v128_xor,
};

use arrayref::array_mut_ref;

use crate::{simd128::safe_arch::store_u32x4, Backend, Buffer, C0, C1, C2, C3};

pub fn detect() -> Option<Backend> {
    Some(Backend::new(fill_buf))
}

pub fn fill_buf(key: &[u32; 8], buf: &mut Buffer) {
    let buf = &mut buf.words;
    let mut ctr = u32x4(0, 1, 2, 3);
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
            x[i] = u32x4_add(x[i], splat(key[i - 4]));
        }

        let group_buf = array_mut_ref![buf, group * 64, 64];
        for (i, &xi) in x.iter().enumerate() {
            store_u32x4(xi, array_mut_ref![group_buf, 4 * i, 4]);
        }

        ctr = u32x4_add(ctr, splat(4));
    }
}

#[inline(always)]
fn quarter_round(x: &mut [v128; 16], a: usize, b: usize, c: usize, d: usize) {
    // a += b; d ^= a; d = rotl(d, 16);
    x[a] = u32x4_add(x[a], x[b]);
    x[d] = v128_xor(x[d], x[a]);
    x[d] = rotl(x[d], 16);

    // c += d; b ^= c; b = rotl(b, 12);
    x[c] = u32x4_add(x[c], x[d]);
    x[b] = v128_xor(x[b], x[c]);
    x[b] = rotl(x[b], 12);

    // a += b; d ^= a; d = rotl(d, 8);
    x[a] = u32x4_add(x[a], x[b]);
    x[d] = v128_xor(x[d], x[a]);
    x[d] = rotl(x[d], 8);

    // c += d; b ^= c; b = rotl(b, 7);
    x[c] = u32x4_add(x[c], x[d]);
    x[b] = v128_xor(x[b], x[c]);
    x[b] = rotl(x[b], 7);
}

#[inline(always)]
fn rotl(x: v128, amt: u32) -> v128 {
    v128_xor(u32x4_shl(x, amt), u32x4_shr(x, 32 - amt))
}
