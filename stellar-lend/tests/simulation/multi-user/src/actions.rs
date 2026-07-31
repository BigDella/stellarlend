use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserAction {
    Deposit,
    Withdraw,
    Borrow,
    Repay,
}

impl UserAction {
    pub fn random<R: Rng>(rng: &mut R) -> Self {
        let choices = [
            UserAction::Deposit,
            UserAction::Withdraw,
            UserAction::Borrow,
            UserAction::Repay,
        ];
        choices[rng.gen_range(0..choices.len())]
    }

    pub fn weighted_random<R: Rng>(rng: &mut R) -> Self {
        // Realistic distribution: more deposits and borrows, fewer withdraws/repays
        let rand = rng.gen_range(0..100);
        match rand {
            0..=35 => UserAction::Deposit,  // 36%
            36..=55 => UserAction::Borrow,  // 20%
            56..=75 => UserAction::Repay,   // 20%
            _ => UserAction::Withdraw,      // 24%
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActionResult {
    pub action: UserAction,
    pub user_id: usize,
    pub timestamp: f64,
    pub amount: i128,
    pub success: bool,
    pub duration_ms: f64,
    pub error_message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_action_generation() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let action = UserAction::random(&mut rng);
            assert!(matches!(
                action,
                UserAction::Deposit
                    | UserAction::Withdraw
                    | UserAction::Borrow
                    | UserAction::Repay
            ));
        }
    }

    #[test]
    fn test_weighted_distribution() {
        let mut rng = rand::thread_rng();
        let mut counts = [0, 0, 0, 0];
        for _ in 0..10000 {
            match UserAction::weighted_random(&mut rng) {
                UserAction::Deposit => counts[0] += 1,
                UserAction::Borrow => counts[1] += 1,
                UserAction::Repay => counts[2] += 1,
                UserAction::Withdraw => counts[3] += 1,
            }
        }
        // Verify distribution is roughly as expected
        assert!(counts[0] > 3000 && counts[0] < 4000); // Deposit ~36%
        assert!(counts[1] > 1500 && counts[1] < 2500); // Borrow ~20%
        assert!(counts[2] > 1500 && counts[2] < 2500); // Repay ~20%
        assert!(counts[3] > 1500 && counts[3] < 2500); // Withdraw ~24%
    }
}
