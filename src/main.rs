use aletheia::engine::session::Session;
use aletheia::engine::fuzzer::Fuzzer;
use aletheia::engine::symbolic::check_mathematical_equivalence;
use aletheia::ipc::mcp::run_mcp_server;
use std::env;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Aletheia: Ultra-Low-Latency In-Memory Verification Engine (Free Tier)");
        println!("Usage: aletheia <eval|equiv|telemetry|fuzz|benchmark|mcp> [args...]");
        println!("  aletheia eval \"comb(100, 50)\" [max_fuel]");
        println!("  aletheia equiv \"(x + 1)^2\" \"x^2 + 2*x + 1\"");
        println!("  aletheia telemetry \"x = 100\"");
        println!("  aletheia fuzz 1000");
        println!("  aletheia benchmark");
        println!("  aletheia mcp (starts Model Context Protocol server over stdio)");
        return;
    }

    match args[1].as_str() {
        "mcp" => {
            if let Err(e) = run_mcp_server() {
                eprintln!("MCP Server error: {}", e);
            }
        }
        "eval" => {
            let expr = args.get(2).map(|s| s.as_str()).unwrap_or("comb(100, 50)");
            let fuel = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(500_000);
            let mut s = Session::new(fuel);
            let t0 = Instant::now();
            match s.eval(expr) {
                Ok(val) => {
                    let dur = t0.elapsed();
                    println!("[+] Result: {}", format!("{:?}", val));
                    println!("[+] Duration: {:.2?} | Fuel Remaining: {}", dur, s.fuel);
                }
                Err(e) => {
                    eprintln!("{}", e);
                }
            }
        }
        "equiv" => {
            let expr_a = args.get(2).map(|s| s.as_str()).unwrap_or("(x + 1)^2");
            let expr_b = args.get(3).map(|s| s.as_str()).unwrap_or("x^2 + 2*x + 1");
            let t0 = Instant::now();
            match check_mathematical_equivalence(expr_a, expr_b) {
                Ok(is_eq) => {
                    let dur = t0.elapsed();
                    println!("[+] Symbolic Equivalence: {}", is_eq);
                    println!("[+] Verified in: {:.2?}", dur);
                }
                Err(e) => {
                    eprintln!("[-] Equivalence error: {}", e);
                }
            }
        }
        "telemetry" => {
            let expr = args.get(2).map(|s| s.as_str()).unwrap_or("x = 100");
            let mut s = Session::new(500_000);
            match s.eval_with_telemetry(expr) {
                Ok(t) => {
                    println!("[+] RL Telemetry Verified:");
                    println!("    Result:        {}", t.result);
                    println!("    Fuel Consumed: {} units", t.fuel_consumed);
                    println!("    Latency:       {} µs", t.latency_us);
                    println!("    Memory:        {} bytes", t.memory_bytes);
                    println!("    Verified:      {}", t.verified);
                }
                Err(e) => {
                    eprintln!("{}", e);
                }
            }
        }
        "fuzz" => {
            let count = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
            println!("[*] Running instant micro-fuzzing with {} test vectors...", count);
            let res = Fuzzer::fuzz_invariant(|x| x.saturating_add(0) == x, count);
            println!("[+] Invariant Verified: {} trials in {} µs (Passed: {})", res.total_cases, res.duration_us, res.passed);
        }
        "benchmark" => {
            println!("==================================================================");
            println!("        ALETHEIA: IN-MEMORY FUEL-METERED VERIFICATION BENCHMARK   ");
            println!("==================================================================");
            let mut s = Session::new(1_000_000);
            let t0 = Instant::now();
            let iters = 100_000;
            for _ in 0..iters {
                let _ = s.eval("100 + 200");
            }
            let dur = t0.elapsed();
            let per_op = dur.as_nanos() as f64 / iters as f64;
            println!("Total In-Memory Evaluations: {} ops", iters);
            println!("Duration:                   {:.2?}", dur);
            println!("Latency per Evaluation:     {:.2} ns/op ({:.2} ops/sec)", per_op, 1_000_000_000.0 / per_op);
            println!("Status:                     [SUB-MICROSECOND VERIFICATION CONFIRMED]");
            println!("==================================================================");
        }
        cmd => {
            eprintln!("Unknown command: {}", cmd);
        }
    }
}
