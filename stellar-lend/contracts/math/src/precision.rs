use soroban_sdk::{contracttype, Env};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PrecisionLoss {
    pub operation: u32,
    pub input_a: i128,
    pub input_b: i128,
    pub result: i128,
    pub lost_amount: i128,
    pub timestamp: u64,
}

impl PrecisionLoss {
    pub fn new_div(env: &Env, a: i128, b: i128, result: i128, lost: i128) -> Self {
        Self {
            operation: 0,
            input_a: a,
            input_b: b,
            result,
            lost_amount: lost,
            timestamp: env.ledger().timestamp(),
        }
    }

    pub fn new_mul_div(env: &Env, a: i128, b: i128, _denom: i128, result: i128, lost: i128) -> Self {
        Self {
            operation: 1,
            input_a: a,
            input_b: b,
            result,
            lost_amount: lost,
            timestamp: env.ledger().timestamp(),
        }
    }
}

pub struct PrecisionTracker {
    logs: soroban_sdk::Vec<PrecisionLoss>,
    total_loss: i128,
    max_loss: i128,
}

impl PrecisionTracker {
    pub fn new(env: &Env) -> Self {
        Self {
            logs: soroban_sdk::Vec::new(env),
            total_loss: 0,
            max_loss: 0,
        }
    }

    pub fn track_division(&mut self, env: &Env, a: i128, b: i128, result: i128) {
        if b == 0 {
            return;
        }
        let _expected = a / b;
        let lost = if a >= 0 && result >= 0 {
            a - result * b
        } else {
            (a - result * b).abs()
        };
        if lost > 0 {
            let entry = PrecisionLoss::new_div(env, a, b, result, lost);
            self.logs.push_back(entry);
            self.total_loss = self.total_loss.saturating_add(lost);
            if lost > self.max_loss {
                self.max_loss = lost;
            }
        }
    }

    pub fn track_mul_div(
        &mut self,
        env: &Env,
        a: i128,
        b: i128,
        denominator: i128,
        result: i128,
    ) {
        if denominator == 0 {
            return;
        }
        let product = a.saturating_mul(b);
        let lost = (product - result.saturating_mul(denominator)).abs();
        if lost > 0 {
            let entry = PrecisionLoss::new_mul_div(env, a, b, denominator, result, lost);
            self.logs.push_back(entry);
            self.total_loss = self.total_loss.saturating_add(lost);
            if lost > self.max_loss {
                self.max_loss = lost;
            }
        }
    }

    pub fn total_loss(&self) -> i128 {
        self.total_loss
    }

    pub fn max_loss(&self) -> i128 {
        self.max_loss
    }

    pub fn log_count(&self) -> u32 {
        self.logs.len() as u32
    }

    pub fn logs(&self) -> &soroban_sdk::Vec<PrecisionLoss> {
        &self.logs
    }

    pub fn clear(&mut self) {
        self.logs = soroban_sdk::Vec::new(self.logs.env());
        self.total_loss = 0;
        self.max_loss = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn track_division_captures_loss() {
        let env = Env::default();
        let mut tracker = PrecisionTracker::new(&env);
        tracker.track_division(&env, 10, 3, 3);
        assert!(tracker.total_loss() > 0);
        assert_eq!(tracker.log_count(), 1);
    }

    #[test]
    fn track_division_exact_no_loss() {
        let env = Env::default();
        let mut tracker = PrecisionTracker::new(&env);
        tracker.track_division(&env, 10, 2, 5);
        assert_eq!(tracker.total_loss(), 0);
        assert_eq!(tracker.log_count(), 0);
    }

    #[test]
    fn track_mul_div_captures_loss() {
        let env = Env::default();
        let mut tracker = PrecisionTracker::new(&env);
        tracker.track_mul_div(&env, 10, 10, 3, 33);
        assert!(tracker.total_loss() >= 0);
    }

    #[test]
    fn track_max_loss_tracks_peak() {
        let env = Env::default();
        let mut tracker = PrecisionTracker::new(&env);
        tracker.track_division(&env, 100, 3, 33);
        assert_eq!(tracker.max_loss(), 1);
    }

    #[test]
    fn clear_resets_state() {
        let env = Env::default();
        let mut tracker = PrecisionTracker::new(&env);
        tracker.track_division(&env, 10, 3, 3);
        assert_eq!(tracker.log_count(), 1);
        tracker.clear();
        assert_eq!(tracker.log_count(), 0);
        assert_eq!(tracker.total_loss(), 0);
        assert_eq!(tracker.max_loss(), 0);
    }
}
