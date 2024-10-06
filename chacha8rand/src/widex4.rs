use crate::{Buffer, C0, C1, C2, C3};
use arrayref::array_mut_ref;
use wide::u32x4;

pub fn fill_buf(key: &[u32; 8], buf: &mut Buffer) {
    let buf = &mut buf.words;
    for i in 0..4 {
        quad_block(key, i, array_mut_ref![buf, i * 64, 64]);
    }
}

// This is a macro instead of a function because that makes the invocations looks more like the
// spec, and doesn't require any borrow checker workarounds.
#[rustfmt::skip]
macro_rules! quarter_round {
    ($a: expr, $b: expr, $c: expr, $d: expr) => {
        $a += $b; $d ^= $a; $d = rotate_left($d, 16);
        $c += $d; $b ^= $c; $b = rotate_left($b, 12);
        $a += $b; $d ^= $a; $d = rotate_left($d, 8);
        $c += $d; $b ^= $c; $b = rotate_left($b, 7);
    };
}

fn rotate_left(x: u32x4, d: u32) -> u32x4 {
    (x << d) | (x >> (32 - d))
}

fn quad_block(key: &[u32; 8], i: usize, buf: &mut [u32; 64]) {
    assert!(i < 4);
    let ctr = (i * 4) as u32;
    let ctr = u32x4::new([ctr, ctr + 1, ctr + 2, ctr + 3]);

    let splat = u32x4::splat;
    #[rustfmt::skip]
    let mut x = [
        splat(C0),     splat(C1),     splat(C2),     splat(C3),
        splat(key[0]), splat(key[1]), splat(key[2]), splat(key[3]),
        splat(key[4]), splat(key[5]), splat(key[6]), splat(key[7]),
        ctr,                 splat(0),      splat(0),      splat(0)
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

    for i in 4..12 {
        x[i] += splat(key[i - 4]);
    }
    // TODO: do we need to worry about endianness here?
    for i in 0..16 {
        buf[i * 4..][..4].copy_from_slice(x[i].as_array_ref());
    }
}
