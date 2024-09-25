/// # Safety
///
/// Requires AVX2 target feature. No other safety requirements.
#[target_feature(enable = "avx2")]
pub unsafe fn fill_buf(key: &[u32; 8], buf: &mut [u32; 256]) {
    todo!()
}
