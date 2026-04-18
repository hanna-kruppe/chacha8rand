#[inline]
pub(crate) fn array_chunks_mut<const C: usize, const N: usize>(
    a: &mut [u8; N],
) -> impl Iterator<Item = &mut [u8; C]> {
    const { assert!(N.is_multiple_of(C)) }
    a.chunks_exact_mut(C).map(|chunk| chunk.try_into().unwrap())
}

#[inline]
pub(crate) fn slice_array_mut<const N: usize>(a: &mut [u8], start: usize) -> &mut [u8; N] {
    (&mut a[start..][..N]).try_into().unwrap()
}

#[inline]
pub(crate) fn slice_array<const N: usize>(a: &[u8], start: usize) -> &[u8; N] {
    a[start..][..N].try_into().unwrap()
}
