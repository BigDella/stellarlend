//! # Gas Optimization Tracking Harness
//!
//! Minimal harness for tracking gas costs per function and operation.
//! Enables gas golf competitions and regression detection.
//!
//! ## Usage
//!
//! ```rust
//! let mut harness = GasHarness::new();
//! harness.start("deposit");
//! // ... function logic ...
//! let metrics = harness.stop();
//! ```

#[cfg(test)]
mod gas_tracking {
    use std::collections::BTreeMap;

    #[derive(Clone, Debug)]
    pub struct GasMetric {
        pub function: String,
        pub cost: u64,
        pub iterations: u32,
    }

    pub struct GasHarness {
        metrics: BTreeMap<String, Vec<u64>>,
        current: Option<String>,
    }

    impl GasHarness {
        pub fn new() -> Self {
            GasHarness {
                metrics: BTreeMap::new(),
                current: None,
            }
        }

        pub fn start(&mut self, function: &str) {
            self.current = Some(function.to_string());
        }

        pub fn stop(&mut self) -> GasMetric {
            let fn_name = self.current.take().expect("no function started");
            // Placeholder: actual implementation would hook into Soroban gas metering
            let cost = 0u64;

            self.metrics
                .entry(fn_name.clone())
                .or_insert_with(Vec::new)
                .push(cost);

            GasMetric {
                function: fn_name,
                cost,
                iterations: 1,
            }
        }

        pub fn leaderboard(&self) -> Vec<(String, u64)> {
            self.metrics
                .iter()
                .map(|(k, v)| {
                    let avg = if v.is_empty() {
                        0
                    } else {
                        v.iter().sum::<u64>() / v.len() as u64
                    };
                    (k.clone(), avg)
                })
                .collect()
        }
    }

    #[test]
    fn test_gas_harness_basic() {
        let mut harness = GasHarness::new();
        harness.start("deposit");
        harness.stop();
        let board = harness.leaderboard();
        assert!(board.iter().any(|(f, _)| f == "deposit"));
    }
}
