use std::array;

mod guts;
#[cfg(test)]
mod tests;

pub struct ChaCha8 {
    seed: [u32; 8],
    i: usize,
    buf: [u32; 256],
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
        let mut this = Self {
            seed: seed.0,
            i: 0,
            buf: [0; 256],
        };
        this.refill_buf();
        this
    }

    fn refill_buf(&mut self) {
        guts::fill_buf(&self.seed, &mut self.buf);
    }

    pub fn next_u32(&mut self) -> u32 {
        let result = self.buf[self.i];
        self.i += 1;
        let new_seed_start = self.buf.len() - self.seed.len();
        if self.i == new_seed_start {
            self.i = 0;
            self.seed.copy_from_slice(&self.buf[new_seed_start..]);
            self.refill_buf();
        }
        result
    }
}
