use core::arch::wasm32::{u32x4, u32x4_add, u32x4_shl, u32x4_shr, v128, v128_xor};

use arrayref::array_mut_ref;

use crate::{
    common_guts::{eight_rounds, init_state},
    simd128::safe_arch::{splat, store_as_u8x16},
    Backend, Buffer,
};

pub fn detect() -> Option<Backend> {
    Some(Backend::new(fill_buf))
}

pub fn fill_buf(key: &[u32; 8], buf: &mut Buffer) {
    let buf = &mut buf.bytes;
    let mut ctr = u32x4(0, 1, 2, 3);
    for group in 0..4 {
        let mut x = init_state(ctr, key, splat);

        eight_rounds(&mut x, quarter_round);

        for i in 4..12 {
            x[i] = u32x4_add(x[i], splat(key[i - 4]));
        }

        let group_buf = array_mut_ref![buf, group * 256, 256];
        for (i, &xi) in x.iter().enumerate() {
            store_as_u8x16(xi, array_mut_ref![group_buf, 16 * i, 16]);
        }

        ctr = u32x4_add(ctr, splat(4));
    }
}

#[inline(always)]
fn quarter_round([mut a, mut b, mut c, mut d]: [v128; 4]) -> [v128; 4] {
    a = u32x4_add(a, b);
    d = v128_xor(d, a);
    d = rotl(d, 16);

    c = u32x4_add(c, d);
    b = v128_xor(b, c);
    b = rotl(b, 12);

    a = u32x4_add(a, b);
    d = v128_xor(d, a);
    d = rotl(d, 8);

    c = u32x4_add(c, d);
    b = v128_xor(b, c);
    b = rotl(b, 7);

    [a, b, c, d]
}

#[inline(always)]
fn rotl(x: v128, amt: u32) -> v128 {
    v128_xor(u32x4_shl(x, amt), u32x4_shr(x, 32 - amt))
}
