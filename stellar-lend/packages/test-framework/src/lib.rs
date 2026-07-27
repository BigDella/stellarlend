use serde::{Deserialize, Serialize};
use soroban_sdk::Env;

pub mod edge_cases;
pub mod fixtures;
pub mod gas_benchmark;
pub mod helpers;
pub mod scenarios;

pub use edge_cases::{EdgeCase, EdgeCaseCatalog};
pub use fixtures::{ContractFixture, FixtureBuilder};
pub use gas_benchmark::{GasBenchmark, GasReport};
pub use helpers::*;
pub use scenarios::{Scenario, ScenarioRunner};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TestCategory {
    Unit,
    Integration,
    Fuzz,
    GasBenchmark,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestConfig {
    pub network: String,
    pub admin: String,
    pub governance: String,
    pub oracle_addresses: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub category: TestCategory,
    pub passed: bool,
    pub message: String,
    pub gas_used: Option<u64>,
}

pub trait TestCase {
    fn name(&self) -> &str;
    fn category(&self) -> TestCategory;
    fn setup(&mut self, env: &Env);
    fn run(&mut self, env: &Env) -> Result<(), String>;
    fn teardown(&mut self, env: &Env);
}

pub struct TestSuite {
    pub name: String,
    pub tests: Vec<Box<dyn TestCase>>,
}

impl TestSuite {
    pub fn new(name: &str) -> Self {
        TestSuite {
            name: name.to_string(),
            tests: Vec::new(),
        }
    }

    pub fn add_test(&mut self, test: Box<dyn TestCase>) {
        self.tests.push(test);
    }

    pub fn run(&mut self, env: &Env) -> Vec<TestResult> {
        let mut results = Vec::new();

        for test_case in self.tests.iter_mut() {
            test_case.setup(env);

            let result = match test_case.run(env) {
                Ok(()) => TestResult {
                    name: test_case.name().to_string(),
                    category: test_case.category(),
                    passed: true,
                    message: "Passed".to_string(),
                    gas_used: None,
                },
                Err(e) => TestResult {
                    name: test_case.name().to_string(),
                    category: test_case.category(),
                    passed: false,
                    message: e,
                    gas_used: None,
                },
            };

            test_case.teardown(env);
            results.push(result);
        }

        results
    }
}

#[derive(Clone, Debug)]
pub struct NamedTest<F>
where
    F: FnMut(&Env) -> Result<(), String>,
{
    name: String,
    category: TestCategory,
    run_fn: F,
}

impl<F> NamedTest<F>
where
    F: FnMut(&Env) -> Result<(), String>,
{
    pub fn new(name: &str, category: TestCategory, run_fn: F) -> Self {
        Self {
            name: name.to_string(),
            category,
            run_fn,
        }
    }
}

impl<F> TestCase for NamedTest<F>
where
    F: FnMut(&Env) -> Result<(), String>,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn category(&self) -> TestCategory {
        self.category
    }

    fn setup(&mut self, _env: &Env) {}

    fn run(&mut self, env: &Env) -> Result<(), String> {
        (self.run_fn)(env)
    }

    fn teardown(&mut self, _env: &Env) {}
}
