use crate::error::UxdError;
use anchor_lang::prelude::*;

pub fn compute_decrease(before: u64, after: u64) -> Result<u64> {
    Ok(checked_sub(before, after)?)
}
