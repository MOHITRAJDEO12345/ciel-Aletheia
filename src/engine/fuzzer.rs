use std::time::Instant;

pub struct FuzzResult {
    pub total_cases: usize,
    pub passed: bool,
    pub duration_us: u128,
    pub counter_example: Option<String>,
}

pub struct Fuzzer;

impl Fuzzer {
    /// Fuzz an invariant assertion against 1,000 auto-generated boundary test cases in < 2ms.
    pub fn fuzz_invariant<F>(predicate: F, total_trials: usize) -> FuzzResult 
    where
        F: Fn(i64) -> bool,
    {
        let t0 = Instant::now();
        // Standard edge vectors: 0, 1, -1, int64 limits, powers of 2, boundary primes
        let base_edges = vec![
            0, 1, -1, 2, -2,
            i64::MAX, i64::MIN,
            i64::MAX - 1, i64::MIN + 1,
            255, 256, 65535, 65536,
            2147483647, -2147483648,
        ];

        let mut current_val = 1337i64;
        for i in 0..total_trials {
            let test_val = if i < base_edges.len() {
                base_edges[i]
            } else {
                // High-speed LCG pseudo-random generator
                current_val = current_val.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                current_val
            };

            if !predicate(test_val) {
                return FuzzResult {
                    total_cases: i + 1,
                    passed: false,
                    duration_us: t0.elapsed().as_micros(),
                    counter_example: Some(format!("Counter-example found at trial {}: input={}", i, test_val)),
                };
            }
        }

        FuzzResult {
            total_cases: total_trials,
            passed: true,
            duration_us: t0.elapsed().as_micros(),
            counter_example: None,
        }
    }

    /// Fuzz an expression invariant (e.g. "x + 0 == x" or "x * 2 == x + x")
    pub fn fuzz_expression(expr: &str, total_trials: usize) -> FuzzResult {
        let t0 = Instant::now();
        let base_edges = vec![
            0, 1, -1, 2, -2,
            100, -100,
            255, 256, 65535, 65536,
            2147483647, -2147483648,
        ];
        let mut current_val = 1337i64;
        let mut session = crate::engine::session::Session::new(100_000);

        for i in 0..total_trials {
            let test_val = if i < base_edges.len() {
                base_edges[i]
            } else {
                current_val = current_val.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                current_val % 1_000_000
            };

            session.fuel = 100_000;
            session.env.insert("x".to_string(), crate::engine::bytecode::StackVal::Int(test_val));
            let passed = match session.eval(expr) {
                Ok(crate::engine::session::Value::Boolean(b)) => b,
                Ok(crate::engine::session::Value::Integer(v)) => v != 0,
                Ok(_) => true,
                Err(_) => false,
            };

            if !passed {
                return FuzzResult {
                    total_cases: i + 1,
                    passed: false,
                    duration_us: t0.elapsed().as_micros(),
                    counter_example: Some(format!("Counter-example found at trial {} with x={}", i, test_val)),
                };
            }
        }

        FuzzResult {
            total_cases: total_trials,
            passed: true,
            duration_us: t0.elapsed().as_micros(),
            counter_example: None,
        }
    }
}
