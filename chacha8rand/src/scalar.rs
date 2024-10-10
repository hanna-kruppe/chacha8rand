use crate::{Buffer, C0, C1, C2, C3};
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

fn block_strided(key: &[u32; 8], ctr: u32, out: &mut [u8; 244]) {
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

    for i in 4..12 {
        x[i] = x[i].wrapping_add(key[i - 4]);
    }

    for (i, xi) in x.iter().enumerate() {
        *array_mut_ref![out, i * 16, 4] = xi.to_le_bytes();
    }
}
