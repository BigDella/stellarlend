use multi_user_simulation::{Simulator, SimulationConfig, SimulationReport};

#[tokio::main]
async fn main() {
    println!("Starting Multi-User Concurrent Simulation...\n");

    let config = SimulationConfig {
        user_count: 50,
        actions_per_user: 100,
        concurrent_users: 10,
        random_seed: None,
    };

    let mut simulator = Simulator::new(config);
    let metrics = simulator.run().await;

    let report = SimulationReport::generate(metrics);
    report.print_summary();
    println!("\nJSON Report:");
    println!("{}", report.to_json());
}
