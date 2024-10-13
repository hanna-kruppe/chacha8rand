use crate::{
    common_guts::{eight_rounds, init_state},
    Backend, Buffer,
};
use arrayref::array_mut_ref;

pub(crate) fn backend() -> Backend {
    Backend::new(fill_buf)
}

#[inline(never)]
fn fill_buf(key: &[u32; 8], buf: &mut Buffer) {
    let buf = &mut buf.bytes;
    for quad in 0..4 {
        let quad_buf = array_mut_ref![buf, quad * 256, 256];
        for block in 0..4 {
            let ctr = (quad * 4 + block) as u32;
            block_strided(key, ctr, array_mut_ref![quad_buf, 4 * block, 256 - 12]);
        }
    }
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
