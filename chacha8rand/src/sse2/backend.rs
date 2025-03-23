use crate::{
    array_ref::array_chunks_mut,
    common_guts::{eight_rounds, init_state},
    sse2::safe_arch::{
        __m128i, add_u32, from_elems, shift_left_u32, shift_right_u32, splat, storeu, xor,
    },
    Backend, Buffer,
};

pub(crate) fn detect() -> Option<Backend> {
    Some(Backend::new(fill_buf))
}

pub(crate) fn fill_buf(key: &[u32; 8], buf: &mut Buffer) {
    let buf = &mut buf.bytes;
    let mut ctr = from_elems([0, 1, 2, 3]);
    for group in array_chunks_mut::<256, 1024>(buf) {
        let mut x = init_state(ctr, key, splat);

        eight_rounds(&mut x, quarter_round);

        for i in 4..12 {
            x[i] = add_u32(x[i], splat(key[i - 4]));
        }

        array_chunks_mut::<16, 256>(group)
            .zip(x)
            .for_each(|(dest, xi)| {
                storeu(xi, dest);
            });

        ctr = add_u32(ctr, splat(4));
    }
}

#[inline(always)]
fn quarter_round([mut a, mut b, mut c, mut d]: [__m128i; 4]) -> [__m128i; 4] {
    a = add_u32(a, b);
    d = xor(d, a);
    d = rotl::<16, 16>(d);

    c = add_u32(c, d);
    b = xor(b, c);
    b = rotl::<12, 20>(b);

    a = add_u32(a, b);
    d = xor(d, a);
    d = rotl::<8, 24>(d);

    c = add_u32(c, d);
    b = xor(b, c);
    b = rotl::<7, 25>(b);

    [a, b, c, d]
}

#[inline(always)]
fn rotl<const SH_LEFT: i32, const SH_RIGHT: i32>(x: __m128i) -> __m128i {
    const {
        assert!(SH_RIGHT == (32 - SH_LEFT));
    }
    xor(shift_left_u32::<SH_LEFT>(x), shift_right_u32::<SH_RIGHT>(x))
}
