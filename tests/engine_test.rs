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
fn test_instant_micro_fuzzer() {
    let res = Fuzzer::fuzz_invariant(|x| x.saturating_add(0) == x, 1000);
    assert!(res.passed);
    assert_eq!(res.total_cases, 1000);
    assert!(res.duration_us < 2000, "Fuzzer took longer than 2ms: {}µs", res.duration_us);
}
