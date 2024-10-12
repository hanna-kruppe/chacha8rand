// The constant words in the first row of the initial state
const C0: u32 = u32::from_le_bytes(*b"expa");
const C1: u32 = u32::from_le_bytes(*b"nd 3");
const C2: u32 = u32::from_le_bytes(*b"2-by");
const C3: u32 = u32::from_le_bytes(*b"te k");

pub(crate) fn init_state<T: Copy>(ctr: T, key: &[u32; 8], splat: impl Fn(u32) -> T) -> [T; 16] {
    #[rustfmt::skip]
    let x = [
        splat(C0),     splat(C1),     splat(C2),     splat(C3),
        splat(key[0]), splat(key[1]), splat(key[2]), splat(key[3]),
        splat(key[4]), splat(key[5]), splat(key[6]), splat(key[7]),
        ctr,           splat(0),      splat(0),      splat(0)
    ];
    x
}

// NB: if `qr` is a closure and dynamic feature detection is involved, that closure really needs to
// be inline(always) so it gets inlined and we get reasonable codegen. (Luckily, `init_state`
// doesn't seem to have the same problem with `splat`. Maybe because splatting is comparatively
// trivial and called less often.)
#[inline(always)]
pub(crate) fn eight_rounds<T: Copy>(x: &mut [T; 16], qr: impl Fn([T; 4]) -> [T; 4]) {
    const ROUNDS: u32 = 8;
    for _ in (0..ROUNDS).step_by(2) {
        // Odd round: columns
        [x[0], x[4], x[8], x[12]] = qr([x[0], x[4], x[8], x[12]]);
        [x[1], x[5], x[9], x[13]] = qr([x[1], x[5], x[9], x[13]]);
        [x[2], x[6], x[10], x[14]] = qr([x[2], x[6], x[10], x[14]]);
        [x[3], x[7], x[11], x[15]] = qr([x[3], x[7], x[11], x[15]]);
        // Even round: diagonals
        [x[0], x[5], x[10], x[15]] = qr([x[0], x[5], x[10], x[15]]);
        [x[1], x[6], x[11], x[12]] = qr([x[1], x[6], x[11], x[12]]);
        [x[2], x[7], x[8], x[13]] = qr([x[2], x[7], x[8], x[13]]);
        [x[3], x[4], x[9], x[14]] = qr([x[3], x[4], x[9], x[14]]);
    }
}
