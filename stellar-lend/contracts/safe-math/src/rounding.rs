use crate::error::MathError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundingMode {
    Down,
    Up,
    Nearest,
}

pub fn round_down(value: i128, _scale: i128) -> i128 {
    value
}

pub fn round_up(value: i128, scale: i128) -> Result<i128, MathError> {
    if scale <= 0 {
        return Ok(value);
    }
    let remainder = value % scale;
    if remainder == 0 {
        return Ok(value);
    }
    value
        .checked_add(scale - remainder)
        .ok_or(MathError::Overflow)
}

pub fn round_nearest(value: i128, scale: i128) -> Result<i128, MathError> {
    if scale <= 0 {
        return Ok(value);
    }
    let half = scale / 2;
    let remainder = value % scale;
    if remainder >= half {
        round_up(value, scale)
    } else {
        Ok(value - remainder)
    }
}
