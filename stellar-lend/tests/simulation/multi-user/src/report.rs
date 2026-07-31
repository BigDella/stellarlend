use crate::metrics::SimulationMetrics;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SimulationReport {
    pub title: String,
    pub timestamp: String,
    pub metrics: SimulationMetrics,
    pub summary: String,
    pub recommendations: Vec<String>,
}

impl SimulationReport {
    pub fn generate(metrics: SimulationMetrics) -> Self {
        let summary = format!(
            "Multi-User Concurrent Simulation Report\n\
             Total Transactions: {}\n\
             Successful: {} ({:.2}%)\n\
             Failed: {}\n\
             Throughput: {:.2} TPS\n\
             Duration: {:.2}s\n\
             State Consistency: {}\n",
            metrics.total_transactions,
            metrics.successful_transactions,
            metrics.success_rate_pct,
            metrics.failed_transactions,
            metrics.total_throughput_tps,
            metrics.total_duration_s,
            if metrics.state_consistency_verified {
                "VERIFIED"
            } else {
                "FAILED"
            }
        );

        let mut recommendations = Vec::new();

        if metrics.success_rate_pct < 95.0 {
            recommendations.push(format!(
                "Success rate is {:.2}%. Consider investigating failure causes.",
                metrics.success_rate_pct
            ));
        }

        if !metrics.bottlenecks.is_empty() {
            recommendations.push("Identified bottlenecks:".to_string());
            for (op, count) in &metrics.bottlenecks {
                recommendations.push(format!("  - {}: {} occurrences", op, count));
            }
        }

        if !metrics.state_consistency_verified {
            recommendations.push("State consistency check FAILED. Investigate invariant violations.".to_string());
            for error in &metrics.consistency_errors {
                recommendations.push(format!("  - {}", error));
            }
        }

        if metrics.total_throughput_tps < 10.0 {
            recommendations.push(format!(
                "Throughput is low ({:.2} TPS). Consider optimizing critical paths.",
                metrics.total_throughput_tps
            ));
        }

        SimulationReport {
            title: "Multi-User Concurrent Interaction Simulation".to_string(),
            timestamp: "simulation_report".to_string(),
            metrics,
            summary,
            recommendations,
        }
    }

    pub fn to_json(&self) -> String {
        format!(
            r#"{{
  "report_title": "{}",
  "timestamp": "{}",
  "metrics": {{
    "total_transactions": {},
    "successful_transactions": {},
    "failed_transactions": {},
    "success_rate_pct": {:.2},
    "throughput_tps": {:.2},
    "duration_s": {:.2},
    "state_consistency_verified": {}
  }},
  "recommendations": [
    {}
  ]
}}"#,
            self.title,
            self.timestamp,
            self.metrics.total_transactions,
            self.metrics.successful_transactions,
            self.metrics.failed_transactions,
            self.metrics.success_rate_pct,
            self.metrics.total_throughput_tps,
            self.metrics.total_duration_s,
            self.metrics.state_consistency_verified,
            self.recommendations
                .iter()
                .map(|r| format!(r#""{}""#, r.replace('"', "\\\"")))
                .collect::<Vec<_>>()
                .join(",\n    ")
        )
    }

    pub fn print_summary(&self) {
        println!("\n{}", "=".repeat(70));
        println!(" {}", self.title);
        println!(" {}", self.timestamp);
        println!("{}\n", "=".repeat(70));
        println!("{}", self.summary);

        if !self.recommendations.is_empty() {
            println!("\nRecommendations:");
            for (i, rec) in self.recommendations.iter().enumerate() {
                println!("  {}. {}", i + 1, rec);
            }
        }
        println!("{}\n", "=".repeat(70));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_generation() {
        let mut metrics = SimulationMetrics::new();
        metrics.total_transactions = 1000;
        metrics.successful_transactions = 950;
        metrics.failed_transactions = 50;
        metrics.total_duration_s = 10.0;
        metrics.state_consistency_verified = true;
        metrics.calculate_stats();

        let report = SimulationReport::generate(metrics);
        assert!(!report.summary.is_empty());
        assert_eq!(report.metrics.total_transactions, 1000);
    }

    #[test]
    fn test_report_json_serialization() {
        let mut metrics = SimulationMetrics::new();
        metrics.total_transactions = 500;
        metrics.successful_transactions = 490;
        metrics.failed_transactions = 10;
        metrics.total_duration_s = 5.0;
        metrics.state_consistency_verified = true;
        metrics.calculate_stats();

        let report = SimulationReport::generate(metrics);
        let json = report.to_json();

        assert!(json.contains("\"total_transactions\": 500"));
        assert!(json.contains("\"state_consistency_verified\": true"));
    }
}
