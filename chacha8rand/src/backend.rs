use crate::Buffer;

// Safety invariant: only constructed with functions that are safe to call. Either because it's
// actually a safe function, or because the function only requires certain target features that were
// checked to be available at runtime.
//
// (The latter case is the whole reason why it's an `unsafe` fn to begin with.)
#[derive(Clone, Copy)]
pub struct Backend {
    refill_impl: unsafe fn(&[u32; 8], &mut Buffer),
}

impl Backend {
    pub(crate) fn new(refill_impl: fn(&[u32; 8], &mut Buffer)) -> Self {
        // SAFETY: `refill_impl` is a safe function, so it's always safe to call.
        Backend { refill_impl }
    }

    /// Create a backend from a refill function gated by dynamic feature detection.
    ///
    /// ## Safety
    ///
    /// The given function must be safe to call, as if it was an ordinary `fn(...)` without `unsafe`
    /// qualifier. For the intended use case of runtime `target_feature` detection, that means the
    /// function must be completely safe *except* for requiring certain target features to be
    /// available, and those target features are in fact available.
    pub(crate) unsafe fn new_unchecked(refill_impl: unsafe fn(&[u32; 8], &mut Buffer)) -> Self {
        // SAFETY: precondition passed on to the caller.
        Self { refill_impl }
    }

    #[doc(hidden)]
    pub fn refill(self, key: &[u32; 8], buf: &mut Buffer) {
        // SAFETY: function is safe to call because that's literally what this type's invariant
        // states.
        unsafe { (self.refill_impl)(key, buf) }
    }
}
