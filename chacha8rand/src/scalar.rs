use crate::{
    array_ref::{array_chunks_mut, slice_array_mut},
    common_guts::{eight_rounds, init_state},
    Backend, Buffer,
};

pub(crate) fn backend() -> Backend {
    Backend::new(fill_buf)
}

#[inline(never)]
fn fill_buf(key: &[u32; 8], buf: &mut Buffer) {
    let buf = &mut buf.bytes;
    for (i, quad) in array_chunks_mut::<256, 1024>(buf).enumerate() {
        for block in 0..4 {
            let ctr = (i * 4 + block) as u32;
            block_strided(key, ctr, slice_array_mut::<{ 256 - 12 }>(quad, 4 * block));
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
        *slice_array_mut::<4>(out, i * 16) = xi.to_le_bytes();
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
