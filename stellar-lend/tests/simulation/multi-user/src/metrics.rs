use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SimulationMetrics {
    pub total_transactions: u64,
    pub successful_transactions: u64,
    pub failed_transactions: u64,
    pub total_throughput_tps: f64,
    pub state_consistency_verified: bool,
    pub consistency_errors: Vec<String>,
    pub bottlenecks: HashMap<String, u64>,
    pub retry_count: u64,
    pub success_rate_pct: f64,
    pub avg_latency_ms: f64,
    pub total_duration_s: f64,
}

impl SimulationMetrics {
    pub fn new() -> Self {
        SimulationMetrics {
            total_transactions: 0,
            successful_transactions: 0,
            failed_transactions: 0,
            total_throughput_tps: 0.0,
            state_consistency_verified: false,
            consistency_errors: Vec::new(),
            bottlenecks: HashMap::new(),
            retry_count: 0,
            success_rate_pct: 0.0,
            avg_latency_ms: 0.0,
            total_duration_s: 0.0,
        }
    }

    pub fn record_success(&mut self, latency_ms: f64) {
        self.total_transactions += 1;
        self.successful_transactions += 1;
    }

    pub fn record_failure(&mut self, error: String) {
        self.total_transactions += 1;
        self.failed_transactions += 1;
        self.consistency_errors.push(error);
    }

    pub fn calculate_stats(&mut self) {
        if self.total_transactions > 0 {
            self.success_rate_pct =
                (self.successful_transactions as f64 / self.total_transactions as f64) * 100.0;
            self.total_throughput_tps = if self.total_duration_s > 0.0 {
                self.total_transactions as f64 / self.total_duration_s
            } else {
                0.0
            };
        }
    }

    pub fn add_bottleneck(&mut self, operation: String) {
        *self.bottlenecks.entry(operation).or_insert(0) += 1;
    }
}

pub struct ThreadSafeMetrics {
    pub transactions: Arc<AtomicU64>,
    pub successes: Arc<AtomicU64>,
    pub failures: Arc<AtomicU64>,
}

impl ThreadSafeMetrics {
    pub fn new() -> Self {
        ThreadSafeMetrics {
            transactions: Arc::new(AtomicU64::new(0)),
            successes: Arc::new(AtomicU64::new(0)),
            failures: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn increment_transaction(&self) {
        self.transactions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_success(&self) {
        self.successes.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_failure(&self) {
        self.failures.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_totals(&self) -> (u64, u64, u64) {
        (
            self.transactions.load(Ordering::Relaxed),
            self.successes.load(Ordering::Relaxed),
            self.failures.load(Ordering::Relaxed),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        let mut metrics = SimulationMetrics::new();
        metrics.record_success(10.5);
        metrics.record_success(15.3);
        metrics.record_failure("test error".to_string());

        assert_eq!(metrics.total_transactions, 3);
        assert_eq!(metrics.successful_transactions, 2);
        assert_eq!(metrics.failed_transactions, 1);
    }

    #[test]
    fn test_metrics_calculation() {
        let mut metrics = SimulationMetrics::new();
        metrics.total_transactions = 100;
        metrics.successful_transactions = 95;
        metrics.total_duration_s = 10.0;
        metrics.calculate_stats();

        assert_eq!(metrics.success_rate_pct, 95.0);
        assert_eq!(metrics.total_throughput_tps, 10.0);
    }

    #[test]
    fn test_thread_safe_metrics() {
        let metrics = ThreadSafeMetrics::new();
        metrics.increment_transaction();
        metrics.increment_transaction();
        metrics.increment_success();
        metrics.increment_failure();

        let (tx, success, failure) = metrics.get_totals();
        assert_eq!(tx, 2);
        assert_eq!(success, 1);
        assert_eq!(failure, 1);
    }
}
