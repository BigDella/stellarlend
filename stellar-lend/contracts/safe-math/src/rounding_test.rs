#[cfg(test)]
mod tests {
    use crate::rounding::{round_down, round_nearest, round_up};

    #[test]
    fn test_round_down() {
        assert_eq!(round_down(17, 10), 17);
        assert_eq!(round_down(20, 10), 20);
        assert_eq!(round_down(0, 10), 0);
    }

    #[test]
    fn test_round_up() {
        assert_eq!(round_up(17, 10).unwrap(), 20);
        assert_eq!(round_up(20, 10).unwrap(), 20);
        assert_eq!(round_up(0, 10).unwrap(), 0);
        assert_eq!(round_up(1, 10).unwrap(), 10);
    }

    #[test]
    fn test_round_nearest() {
        assert_eq!(round_nearest(14, 10).unwrap(), 10);
        assert_eq!(round_nearest(16, 10).unwrap(), 20);
        assert_eq!(round_nearest(15, 10).unwrap(), 20);
        assert_eq!(round_nearest(10, 10).unwrap(), 10);
    }

    #[test]
    fn test_round_up_zero_scale() {
        assert_eq!(round_up(17, 0).unwrap(), 17);
    }

    #[test]
    fn test_round_nearest_zero_scale() {
        assert_eq!(round_nearest(17, 0).unwrap(), 17);
    }
}
