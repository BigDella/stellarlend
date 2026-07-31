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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_down_noop() {
        assert_eq!(round_down(42, 1), 42);
    }

    #[test]
    fn round_up_exact() {
        assert_eq!(round_up(100, 10), Ok(100));
    }

    #[test]
    fn round_up_ceil() {
        assert_eq!(round_up(101, 10), Ok(110));
        assert_eq!(round_up(109, 10), Ok(110));
    }

    #[test]
    fn round_up_zero_scale() {
        assert_eq!(round_up(42, 0), Ok(42));
        assert_eq!(round_up(42, -1), Ok(42));
    }

    #[test]
    fn round_up_overflow() {
        assert_eq!(round_up(i128::MAX, 10), Err(MathError::Overflow));
    }

    #[test]
    fn round_nearest_down() {
        assert_eq!(round_nearest(104, 10), Ok(100));
    }

    #[test]
    fn round_nearest_up() {
        assert_eq!(round_nearest(105, 10), Ok(110));
        assert_eq!(round_nearest(106, 10), Ok(110));
    }

    #[test]
    fn round_nearest_exact() {
        assert_eq!(round_nearest(100, 10), Ok(100));
        assert_eq!(round_nearest(110, 10), Ok(110));
    }

    #[test]
    fn round_nearest_zero_scale() {
        assert_eq!(round_nearest(42, 0), Ok(42));
    }

    #[test]
    fn round_up_for_liabilities() {
        let principal = 1_000_000i128;
        let rate_bps = 500i128;
        let value = principal * rate_bps;
        let rounded_up = round_up(value, 10_000).unwrap();
        let expected = (value + 10_000 - 1) / 10_000 * 10_000;
        assert_eq!(rounded_up, expected);
    }
}
