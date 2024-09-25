use crate::guts;

// Safety invariant: only constructed with functions that are safe to call. Either because it's
// actually a safe function, or because the function only requires certain target features that were
// checked to be available at runtime.
//
// (The latter case is the whole reason why it's an `unsafe` fn to begin with.)
#[derive(Clone, Copy)]
pub struct Backend(unsafe fn(&[u32; 8], &mut [u32; 256]));

impl Backend {
    pub(crate) fn safe(f: fn(&[u32; 8], &mut [u32; 256])) -> Self {
        // Safety: `f` is a safe function.
        Backend(f)
    }

    pub fn scalar() -> Backend {
        Self::safe(guts::scalar::fill_buf)
    }

    pub fn simd128() -> Backend {
        Self::safe(guts::simd128::fill_buf)
    }

    pub fn avx2() -> Option<Self> {
        #[cfg(target_arch = "x86_64")]
        if std::is_x86_feature_detected!("avx2") {
            // Safety: the function needs AVX2 (which is available in this `if`) and has no
            // other safety preconditions.
            return Some(Backend(guts::avx2::fill_buf));
        }
        None
    }

    #[doc(hidden)]
    // This does not actually compute ChaCha8 output, it doesn't refill the buffer at all. Only
    // useful for a benchmark that estimates the baseline overhead of next_u32().
    pub fn totally_wrong_stub_for_testing_that_breaks_everything_if_you_actually_use_it() -> Self {
        Backend::safe(|_key, _seed| {})
    }

    #[doc(hidden)]
    pub fn refill(self, key: &[u32; 8], buf: &mut [u32; 256]) {
        // Safety: function is safe to call because that's literally what this type's invariant
        // states.
        unsafe { (self.0)(key, buf) }
    }
}
