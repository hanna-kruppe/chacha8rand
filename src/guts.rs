use crate::RefillFn;

pub mod scalar;

pub fn select_impl() -> RefillFn {
    scalar::fill_buf
}
