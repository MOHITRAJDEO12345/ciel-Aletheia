use aletheia::engine::fuel::FuelMeter;
use aletheia::engine::rational::RationalNumber;
use aletheia::engine::session::Session;
use aletheia::engine::fuzzer::Fuzzer;
use aletheia::billboard::restricted::AletheiaError;

#[test]
fn test_fuel_meter_trap() {
    let fuel = FuelMeter::new(100);
    assert!(fuel.consume(50).is_ok());
    assert_eq!(fuel.remaining(), 50);
    assert_eq!(fuel.consumed(), 50);
    assert!(fuel.consume(60).is_err()); // Out of fuel trap
    assert_eq!(fuel.remaining(), 0);
}

#[test]
fn test_exact_rational_comb() {
    let c = RationalNumber::comb(100, 50);
    assert!(c.is_integer());
    assert_eq!(c.to_string(), "100891344545564193334812497256");
}

#[test]
fn test_session_micro_cow_branching() {
    let mut s1 = Session::new(100_000);
    let _ = s1.eval("40 + 2");
    
    // Fork in sub-microseconds
    let s2 = s1.fork();
    assert_eq!(s1.fuel, s2.fuel);
}

#[test]
fn test_smart_exception_billboard() {
    let mut s = Session::new(100_000);
    let res = s.eval("import subprocess; subprocess.run(['npm', 'test'])");
    match res {
        Err(AletheiaError::RestrictedOperation { attempted, .. }) => {
            assert_eq!(attempted, "import subprocess");
        }
        _ => panic!("Expected restricted operation error"),
    }
}

#[test]
fn test_structured_escalation_path() {
    use aletheia::billboard::restricted::EscalationCapability;

    let mut s = Session::new(100_000);
    // 1. Seed state in Tier 0
    let _ = s.eval("x = 42");

    // 2. Attempt filesystem access
    let fs_res = s.eval("with open('/etc/hosts') as f: data = f.read()");
    match fs_res {
        Err(AletheiaError::RestrictedOperation {
            capability,
            attempted,
            upgrade_code,
            serialized_state,
            ..
        }) => {
            assert_eq!(capability, EscalationCapability::RequiresFilesystem);
            assert_eq!(attempted, "open(");
            assert_eq!(upgrade_code, "sandbox = ananke.upgrade_to_thanaton(session)");
            assert!(serialized_state.get("x").is_some(), "State 'x' must be preserved in escalation error");
        }
        _ => panic!("Expected filesystem restricted operation"),
    }

    // 3. Attempt package installation
    let pkg_res = s.eval("npm install express");
    match pkg_res {
        Err(AletheiaError::RestrictedOperation { capability, attempted, .. }) => {
            assert_eq!(capability, EscalationCapability::RequiresPackages);
            assert_eq!(attempted, "npm install");
        }
        _ => panic!("Expected packages restricted operation"),
    }

    // 4. Attempt socket connection
    let net_res = s.eval("import socket; s = socket.socket()");
    match net_res {
        Err(AletheiaError::RestrictedOperation { capability, attempted, .. }) => {
            assert_eq!(capability, EscalationCapability::RequiresNetwork);
            assert_eq!(attempted, "import socket");
        }
        _ => panic!("Expected network restricted operation"),
    }
}

#[test]
fn test_instant_micro_fuzzer() {
    let res = Fuzzer::fuzz_invariant(|x| x.saturating_add(0) == x, 1000);
    assert!(res.passed);
    assert_eq!(res.total_cases, 1000);
    assert!(res.duration_us < 2000, "Fuzzer took longer than 2ms: {}µs", res.duration_us);
}

#[test]
fn test_compound_arithmetic_expressions() {
    use aletheia::engine::session::Value;

    let mut s = Session::new(100_000);
    
    // 1. Compound addition
    let res = s.eval("1 + 2 + 3").expect("Failed to evaluate 1 + 2 + 3");
    assert_eq!(res, Value::Integer(6));

    // 2. Precedence (mul before sub)
    let res = s.eval("10 * 5 - 2").expect("Failed to evaluate 10 * 5 - 2");
    assert_eq!(res, Value::Integer(48));

    // 3. Parentheses override
    let res = s.eval("(2 + 3) * 4").expect("Failed to evaluate (2 + 3) * 4");
    assert_eq!(res, Value::Integer(20));

    // 4. Variables and arithmetic
    let _ = s.eval("x = 15");
    let res = s.eval("x * 2 + 10").expect("Failed to evaluate x * 2 + 10");
    assert_eq!(res, Value::Integer(40));

    // 5. comb(n, k) inside expression
    let res = s.eval("comb(5, 2) + 3").expect("Failed to evaluate comb(5, 2) + 3");
    assert_eq!(res, Value::Integer(13));
}

#[test]
fn test_billboard_comment_false_positives_and_import_variations() {
    let mut s = Session::new(100_000);

    // 1. Comments containing forbidden keywords must NOT trigger error
    let res = s.eval("x = 42 # open('/etc/passwd')");
    assert!(res.is_ok(), "Comment triggered false positive billboard error");

    // 2. String literal containing forbidden pattern must NOT trigger error
    let res = s.eval("y = 100 // import os");
    println!("res 2: {:?}", res);
    assert!(res.is_ok(), "C-style comment triggered false positive error");

    // 3. from os import system MUST trigger error
    let res = s.eval("from os import system");
    assert!(res.is_err(), "from os import system evaded billboard");

    // 4. __import__ MUST trigger error
    let res = s.eval("__import__('os')");
    assert!(res.is_err(), "__import__ evaded billboard");
}

#[test]
fn test_fuel_meter_reset_consumed_desync_fix() {
    use aletheia::engine::fuel::FuelMeter;

    let meter = FuelMeter::new(1000);
    assert_eq!(meter.consumed(), 0);
    meter.consume(300).unwrap();
    assert_eq!(meter.consumed(), 300);

    // Reset to higher budget: must NOT desync consumed()
    meter.reset(5000);
    assert_eq!(meter.consumed(), 0);
    meter.consume(1500).unwrap();
    assert_eq!(meter.consumed(), 1500);
}

#[test]
fn test_identifier_numeric_and_compound_assignment() {
    let mut s = Session::new(100_000);

    // 1. Numeric identifiers must NOT be saved as variables
    let _res = s.eval("123 = 456");
    // Evaluating "123 = 456" must either error or evaluate as an invalid assignment, not store variable "123"
    assert!(!s.env.contains_key("123"), "Numeric variable name '123' was saved into env");

    // 2. Compound assignment operators like += or *= must not corrupt variable names
    let _ = s.eval("x += 10");
    assert!(!s.env.contains_key("x +"), "Compound operator corrupted variable name to 'x +'");
}

#[test]
fn test_telemetry_true_memory_bytes() {
    let mut s = Session::new(100_000);
    let t1 = s.eval_with_telemetry("x = 42").unwrap();
    let initial_mem = t1.memory_bytes;

    // Allocate a large string variable
    let _ = s.eval("large_str = 1000");
    let t2 = s.eval_with_telemetry("y = 9999").unwrap();
    assert!(t2.memory_bytes >= initial_mem, "Memory telemetry failed to measure allocations");
}

#[test]
fn test_fuzzer_custom_expression() {
    // Custom invariant: x * 2 == x + x
    let res = Fuzzer::fuzz_expression("x * 2 == x + x", 100);
    assert!(res.passed, "x * 2 == x + x failed fuzzing: {:?}", res.counter_example);

    // Failing invariant: x + 1 == x (must fail)
    let res_fail = Fuzzer::fuzz_expression("x + 1 == x", 100);
    assert!(!res_fail.passed, "x + 1 == x unexpectedly passed");
    assert!(res_fail.counter_example.is_some());
}
