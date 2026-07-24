use soroban_sdk::{contracttype, Env};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PrecisionLoss {
    pub operation: i128,
    pub input_a: i128,
    pub input_b: i128,
    pub result: i128,
    pub lost_amount: i128,
    pub timestamp: u64,
}

pub struct PrecisionTracker {
    logs: soroban_sdk::Vec<PrecisionLoss>,
    total_loss: i128,
}

impl PrecisionTracker {
    pub fn new(env: &Env) -> Self {
        Self {
            logs: soroban_sdk::Vec::new(env),
            total_loss: 0,
        }
    }

    pub fn track_division(&mut self, env: &Env, a: i128, b: i128, result: i128) {
        if b == 0 {
            return;
        }
        let expected = a / b;
        let lost = (a - result * b).abs();
        if lost > 0 {
            let entry = PrecisionLoss {
                operation: 0,
                input_a: a,
                input_b: b,
                result,
                lost_amount: lost,
                timestamp: env.ledger().timestamp(),
            };
            self.logs.push_back(entry);
            self.total_loss += lost;
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
        let lost = ((a * b) - result * denominator).abs();
        if lost > 0 {
            let entry = PrecisionLoss {
                operation: 1,
                input_a: a,
                input_b: b,
                result,
                lost_amount: lost,
                timestamp: env.ledger().timestamp(),
            };
            self.logs.push_back(entry);
            self.total_loss += lost;
        }
    }

    pub fn total_loss(&self) -> i128 {
        self.total_loss
    }

    pub fn log_count(&self) -> u32 {
        self.logs.len() as u32
    }
}
