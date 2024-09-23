#[inline(never)]
pub fn fill_buf(seed: &[u32; 8], buf: &mut [u32; 256]) {
    let mut unpermuted = [0; 256];
    chacha8(seed, &mut unpermuted);
    let mut out_idx = 0;
    for four_blocks in unpermuted.chunks_exact(4 * WORDS_PER_BLOCK) {
        for i in 0..16 {
            buf[out_idx] = four_blocks[i];
            buf[out_idx + 1] = four_blocks[i + 16];
            buf[out_idx + 2] = four_blocks[i + 32];
            buf[out_idx + 3] = four_blocks[i + 48];
            out_idx += 4;
        }
    }
    assert_eq!(out_idx, buf.len());
}

const WORDS_PER_BLOCK: usize = 16;

fn chacha8(key: &[u32; 8], out: &mut [u32]) {
    assert_eq!(out.len() % WORDS_PER_BLOCK, 0);
    for (i, out_block) in out.chunks_exact_mut(WORDS_PER_BLOCK).enumerate() {
        let ctr = u32::try_from(i).unwrap();
        chacha8_block(key, ctr, out_block.try_into().unwrap());
    }
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

// The constant words in the first row of the initial state
const C0: u32 = u32::from_le_bytes(*b"expa");
const C1: u32 = u32::from_le_bytes(*b"nd 3");
const C2: u32 = u32::from_le_bytes(*b"2-by");
const C3: u32 = u32::from_le_bytes(*b"te k");

fn chacha8_block(key: &[u32; 8], ctr: u32, out: &mut [u32; WORDS_PER_BLOCK]) {
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
    out.copy_from_slice(&x);
    for (i, ki) in key.iter().copied().enumerate() {
        out[i + 4] = out[i + 4].wrapping_add(ki);
    }
}
