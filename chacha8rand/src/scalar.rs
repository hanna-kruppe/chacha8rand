use crate::{
    common_guts::{eight_rounds, init_state},
    Buffer,
};
use arrayref::array_mut_ref;

#[inline(never)]
pub fn fill_buf(key: &[u32; 8], buf: &mut Buffer) {
    let buf = &mut buf.bytes;
    quad_block(key, 0, array_mut_ref![buf, 0, 256]);
    quad_block(key, 1, array_mut_ref![buf, 256, 256]);
    quad_block(key, 2, array_mut_ref![buf, 2 * 256, 256]);
    quad_block(key, 3, array_mut_ref![buf, 3 * 256, 256]);
}

fn quad_block(key: &[u32; 8], i: usize, buf: &mut [u8; 256]) {
    assert!(i < 4);
    let ctr = (i * 4) as u32;
    block_strided(key, ctr, array_mut_ref![buf, 0, 256 - 12]);
    block_strided(key, ctr + 1, array_mut_ref![buf, 4, 256 - 12]);
    block_strided(key, ctr + 2, array_mut_ref![buf, 8, 256 - 12]);
    block_strided(key, ctr + 3, array_mut_ref![buf, 12, 256 - 12]);
}

fn block_strided(key: &[u32; 8], ctr: u32, out: &mut [u8; 244]) {
    let mut x = init_state(ctr, key, |n| n);

    eight_rounds(&mut x, quarter_round);

    for i in 4..12 {
        x[i] = x[i].wrapping_add(key[i - 4]);
    }

    for (i, xi) in x.iter().enumerate() {
        *array_mut_ref![out, i * 16, 4] = xi.to_le_bytes();
    }
}

#[inline(always)]
fn quarter_round([mut a, mut b, mut c, mut d]: [u32; 4]) -> [u32; 4] {
    a = a.wrapping_add(b);
    d ^= a;
    d = d.rotate_left(16);

    c = c.wrapping_add(d);
    b ^= c;
    b = b.rotate_left(12);

    a = a.wrapping_add(b);
    d ^= a;
    d = d.rotate_left(8);

    c = c.wrapping_add(d);
    b ^= c;
    b = b.rotate_left(7);

    [a, b, c, d]
}
