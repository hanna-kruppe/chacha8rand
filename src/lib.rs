use std::array;

pub mod guts;
#[cfg(test)]
mod tests;

pub type RefillFn = fn(&[u32; 8], &mut [u32; 256]);

pub struct ChaCha8 {
    seed: [u32; 8],
    i: usize,
    buf: [u32; 256],
    refill: RefillFn,
}

pub struct Seed([u32; 8]);

impl From<[u8; 32]> for Seed {
    fn from(bytes: [u8; 32]) -> Self {
        Self(array::from_fn(|i| {
            u32::from_le_bytes(bytes[4 * i..][..4].try_into().unwrap())
        }))
    }
}

impl From<&[u8; 32]> for Seed {
    fn from(bytes: &[u8; 32]) -> Self {
        Self::from(*bytes)
    }
}

impl ChaCha8 {
    pub fn new(seed: Seed) -> Self {
        Self::new_with_impl(seed, guts::select_impl())
    }

    pub fn new_with_impl(seed: Seed, refill: RefillFn) -> Self {
        let mut this = Self {
            seed: seed.0,
            i: 0,
            buf: [0; 256],
            refill,
        };
        refill(&this.seed, &mut this.buf);
        this
    }

    pub fn next_u32(&mut self) -> u32 {
        let result = self.buf[self.i];
        self.i += 1;
        let new_seed_start = self.buf.len() - self.seed.len();
        if self.i == new_seed_start {
            self.seed.copy_from_slice(&self.buf[new_seed_start..]);
            (self.refill)(&self.seed, &mut self.buf);
            self.i = 0;
        }
        result
    }
}
