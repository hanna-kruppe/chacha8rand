use crate::{
    guts::{C0, C1, C2, C3},
    Buffer,
};
use arrayref::array_mut_ref;

#[inline(never)]
pub fn fill_buf(key: &[u32; 8], buf: &mut Buffer) {
    let buf = &mut buf.words;
    quad_block(key, 0, array_mut_ref![buf, 0, 64]);
    quad_block(key, 1, array_mut_ref![buf, 64, 64]);
    quad_block(key, 2, array_mut_ref![buf, 2 * 64, 64]);
    quad_block(key, 3, array_mut_ref![buf, 3 * 64, 64]);
}

fn quad_block(key: &[u32; 8], i: usize, buf: &mut [u32; 64]) {
    assert!(i < 4);
    let ctr = (i * 4) as u32;
    block_strided(key, ctr, array_mut_ref![buf, 0, 61]);
    block_strided(key, ctr + 1, array_mut_ref![buf, 1, 61]);
    block_strided(key, ctr + 2, array_mut_ref![buf, 2, 61]);
    block_strided(key, ctr + 3, array_mut_ref![buf, 3, 61]);
}

// This is a macro instead of a function because that makes the invocations looks more like the
// spec, and doesn't require any borrow checker workarounds.
#[rustfmt::skip]
macro_rules! quarter_round {
    ($a: expr, $b: expr, $c: expr, $d: expr) => {
        $a = $a.wrapping_add($b); $d ^= $a; $d = $d.rotate_left(16);
        $c = $c.wrapping_add($d); $b ^= $c; $b = $b.rotate_left(12);
        $a = $a.wrapping_add($b); $d ^= $a; $d = $d.rotate_left(8);
        $c = $c.wrapping_add($d); $b ^= $c; $b = $b.rotate_left(7);
    };
}

fn block_strided(key: &[u32; 8], ctr: u32, out: &mut [u32; 61]) {
    #[rustfmt::skip]
    let mut x = [
        C0,     C1,     C2,     C3,
        key[0], key[1], key[2], key[3],
        key[4], key[5], key[6], key[7],
        ctr,    0,      0,      0
    ];

    const ROUNDS: usize = 8;
    for _ in (0..ROUNDS).step_by(2) {
        quarter_round!(x[0], x[4], x[8], x[12]);
        quarter_round!(x[1], x[5], x[9], x[13]);
        quarter_round!(x[2], x[6], x[10], x[14]);
        quarter_round!(x[3], x[7], x[11], x[15]);

        quarter_round!(x[0], x[5], x[10], x[15]);
        quarter_round!(x[1], x[6], x[11], x[12]);
        quarter_round!(x[2], x[7], x[8], x[13]);
        quarter_round!(x[3], x[4], x[9], x[14]);
    }

    for i in 0..4 {
        out[i * 4] = x[i];
    }
    for i in 4..12 {
        out[i * 4] = x[i].wrapping_add(key[i - 4]);
    }
    for i in 12..16 {
        out[i * 4] = x[i];
    }
}
