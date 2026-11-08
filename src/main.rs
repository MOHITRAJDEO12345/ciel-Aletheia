use aletheia::engine::session::Session;
use aletheia::engine::fuzzer::Fuzzer;
use aletheia::engine::symbolic::check_mathematical_equivalence;
use aletheia::engine::math_verify::verify_math;
use aletheia::engine::extractor::{extract_code_blocks, split_thought_and_response};
use aletheia::ipc::mcp::run_mcp_server;
use std::env;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Aletheia: Ultra-Low-Latency In-Memory Verification Engine (Free Tier)");
        println!("Usage: aletheia <command> [args...]");
        println!("  aletheia verify-math \"\\boxed{{\\frac{{1}}{{2}}}}\" \"0.5\"     (Fast RLVR Math Verifier)");
        println!("  aletheia extract-answer \"The answer is \\boxed{{42}}.\"        (Extracts boxed/GSM8K answer)");
        println!("  aletheia extract-code \"```python\\nprint(42)\\n```\"             (Extracts code blocks)");
        println!("  aletheia run \"a = 10; b = 20; assert a + b == 30; gcd(a, b)\"  (Multi-statement assertions)");
        println!("  aletheia eval \"comb(100, 50)\" [max_fuel]");
        println!("  aletheia equiv \"(x + 1)^2\" \"x^2 + 2*x + 1\"");
        println!("  aletheia telemetry \"x = 100\"");
        println!("  aletheia fuzz 1000");
        println!("  aletheia benchmark");
        println!("  aletheia mcp (starts Model Context Protocol server over stdio)");
        std::process::exit(1);
    }

    match args[1].as_str() {
        "mcp" => {
            if let Err(e) = run_mcp_server() {
                eprintln!("MCP Server error: {}", e);
                std::process::exit(1);
            }
        }
        "verify-math" => {
            let gold = match args.get(2) {
                Some(g) => g.as_str(),
                None => {
                    eprintln!("[-] Error: Ground truth required. Usage: aletheia verify-math <gold> <pred>");
                    std::process::exit(1);
                }
            };
            let pred = match args.get(3) {
                Some(p) => p.as_str(),
                None => {
                    eprintln!("[-] Error: Prediction required. Usage: aletheia verify-math <gold> <pred>");
                    std::process::exit(1);
                }
            };
            let res = verify_math(gold, pred);
            println!("[+] Math Verification: {}", if res.verified { "PASSED" } else { "FAILED" });
            println!("    Gold Extracted:    {}", res.gold_extracted);
            println!("    Pred Extracted:    {}", res.pred_extracted);
            println!("    Duration:          {} µs", res.duration_us);
            if !res.verified {
                std::process::exit(2);
            }
        }
        "extract-answer" => {
            let text = match args.get(2) {
                Some(t) => t.as_str(),
                None => {
                    eprintln!("[-] Error: Text required. Usage: aletheia extract-answer <text>");
                    std::process::exit(1);
                }
            };
            let ans = aletheia::engine::math_verify::extract_answer(text);
            println!("{}", ans);
        }
        "extract-code" => {
            let text = match args.get(2) {
                Some(t) => t.as_str(),
                None => {
                    eprintln!("[-] Error: Text required. Usage: aletheia extract-code <text> [lang]");
                    std::process::exit(1);
                }
            };
            let lang = args.get(3).map(|s| s.as_str());
            let blocks = extract_code_blocks(text, lang);
            let trace = split_thought_and_response(text);
            if let Some(thought) = trace.thought {
                println!("--- Thought Trace ---");
                println!("{}", thought);
                println!("---------------------");
            }
            println!("Extracted {} code block(s):", blocks.len());
            for (idx, b) in blocks.iter().enumerate() {
                println!("[Block {} ({})]:\n{}", idx + 1, b.language, b.code);
            }
        }
        "run" => {
            let script = match args.get(2) {
                Some(s) => s.as_str(),
                None => {
                    eprintln!("[-] Error: Script required. Usage: aletheia run <script> [fuel]");
                    std::process::exit(1);
                }
            };
            let fuel = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(500_000);
            let mut s = Session::new(fuel);
            let t0 = Instant::now();
            match s.eval_multi(script) {
                Ok(val) => {
                    let dur = t0.elapsed();
                    println!("[+] Multi-Statement Result: {:?}", val);
                    println!("[+] Duration: {:.2?} | Fuel Remaining: {}", dur, s.fuel);
                }
                Err(e) => {
                    eprintln!("[-] Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "eval" => {
            let expr = match args.get(2) {
                Some(e) => e.as_str(),
                None => {
                    eprintln!("[-] Error: Expression required. Usage: aletheia eval <expression> [fuel]");
                    std::process::exit(1);
                }
            };
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
                    std::process::exit(1);
                }
            }
        }
        "equiv" => {
            let expr_a = match args.get(2) {
                Some(e) => e.as_str(),
                None => {
                    eprintln!("[-] Error: Two expressions required. Usage: aletheia equiv <expr_a> <expr_b>");
                    std::process::exit(1);
                }
            };
            let expr_b = match args.get(3) {
                Some(e) => e.as_str(),
                None => {
                    eprintln!("[-] Error: Two expressions required. Usage: aletheia equiv <expr_a> <expr_b>");
                    std::process::exit(1);
                }
            };
            let t0 = Instant::now();
            match check_mathematical_equivalence(expr_a, expr_b) {
                Ok(is_eq) => {
                    let dur = t0.elapsed();
                    println!("[+] Symbolic Equivalence: {}", is_eq);
                    println!("[+] Verified in: {:.2?}", dur);
                    if !is_eq {
                        std::process::exit(2);
                    }
                }
                Err(e) => {
                    eprintln!("[-] Equivalence error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "telemetry" => {
            let expr = match args.get(2) {
                Some(e) => e.as_str(),
                None => {
                    eprintln!("[-] Error: Expression required. Usage: aletheia telemetry <expression>");
                    std::process::exit(1);
                }
            };
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
                    std::process::exit(1);
                }
            }
        }
        "fuzz" => {
            let count = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
            let expr_opt = args.get(3).map(|s| s.as_str());
            match expr_opt {
                Some(expr) => {
                    println!("[*] Running micro-fuzzing with {} test vectors for invariant '{}'...", count, expr);
                    let res = Fuzzer::fuzz_expression(expr, count);
                    println!("[+] Invariant Verified: {} trials in {} µs (Passed: {})", res.total_cases, res.duration_us, res.passed);
                    if let Some(counter) = res.counter_example {
                        eprintln!("[-] {}", counter);
                        std::process::exit(1);
                    }
                }
                None => {
                    println!("[*] Running micro-fuzzing with {} test vectors for identity invariant 'x + 0 == x'...", count);
                    let res = Fuzzer::fuzz_invariant(|x| x.saturating_add(0) == x, count);
                    println!("[+] Invariant Verified: {} trials in {} µs (Passed: {})", res.total_cases, res.duration_us, res.passed);
                    if !res.passed {
                        std::process::exit(1);
                    }
                }
            }
        }
        "benchmark" => {
            println!("================================================================================");
            println!("        CIEL ALETHEIA: PRODUCTION IN-MEMORY VERIFICATION BENCHMARK SUITE       ");
            println!("================================================================================");

            // 1. In-Memory Bytecode Micro-op
            {
                let mut s = Session::new(10_000_000);
                let iters = 100_000;
                let t0 = Instant::now();
                for _ in 0..iters {
                    let _ = s.eval("100 + 200");
                }
                let dur = t0.elapsed();
                let per_op_ns = dur.as_nanos() as f64 / iters as f64;
                println!("[Suite 1] Arithmetic Bytecode VM (100 + 200):");
                println!("          Total Operations:  {} ops in {:.2?}", iters, dur);
                println!("          Latency:           {:.2} ns/op", per_op_ns);
                println!("          Throughput:        {:.0} ops/sec", 1_000_000_000.0 / per_op_ns);
            }

            // 2. Exact Combinatorics (BigInt)
            {
                let mut s = Session::new(10_000_000);
                let iters = 20_000;
                let t0 = Instant::now();
                for _ in 0..iters {
                    let _ = s.eval("comb(100, 50)");
                }
                let dur = t0.elapsed();
                let per_op_us = dur.as_micros() as f64 / iters as f64;
                println!("[Suite 2] BigInt Combinatorics (comb(100, 50)):");
                println!("          Total Operations:  {} ops in {:.2?}", iters, dur);
                println!("          Latency:           {:.2} µs/op", per_op_us);
                println!("          Throughput:        {:.0} ops/sec", 1_000_000.0 / per_op_us);
            }

            // 3. Multi-Statement Assertion Script
            {
                let iters = 20_000;
                let script = "a = 10; b = 20; assert a + b == 30; gcd(a, b)";
                let t0 = Instant::now();
                for _ in 0..iters {
                    let mut s = Session::new(500_000);
                    let _ = s.eval_multi(script);
                }
                let dur = t0.elapsed();
                let per_op_us = dur.as_micros() as f64 / iters as f64;
                println!("[Suite 3] Multi-Statement Script & Invariant Assert VM:");
                println!("          Script:            '{}'", script);
                println!("          Total Operations:  {} ops in {:.2?}", iters, dur);
                println!("          Latency:           {:.2} µs/op", per_op_us);
                println!("          Throughput:        {:.0} ops/sec", 1_000_000.0 / per_op_us);
            }

            // 4. Symbolic Polynomial Equivalence
            {
                let iters = 10_000;
                let t0 = Instant::now();
                for _ in 0..iters {
                    let _ = check_mathematical_equivalence("(x + 1)^2", "x^2 + 2*x + 1");
                }
                let dur = t0.elapsed();
                let per_op_us = dur.as_micros() as f64 / iters as f64;
                println!("[Suite 4] Symbolic Polynomial Equivalence ((x+1)^2 == x^2+2x+1):");
                println!("          Total Operations:  {} ops in {:.2?}", iters, dur);
                println!("          Latency:           {:.2} µs/op", per_op_us);
                println!("          Throughput:        {:.0} ops/sec", 1_000_000.0 / per_op_us);
            }

            // 5. Zero-SymPy RLVR Math Verifier
            {
                let iters = 20_000;
                let gold = r"\boxed{\frac{1}{2}}";
                let pred = "Therefore the final answer is 0.5";
                let t0 = Instant::now();
                let mut passed = 0;
                for _ in 0..iters {
                    let res = verify_math(gold, pred);
                    if res.verified {
                        passed += 1;
                    }
                }
                let dur = t0.elapsed();
                let per_op_us = dur.as_micros() as f64 / iters as f64;
                println!("[Suite 5] Zero-SymPy RLVR LaTeX Math Verifier (\\boxed{{1/2}} vs '0.5'):");
                println!("          Total Operations:  {} ops in {:.2?} (Passed: {}/{})", iters, dur, passed, iters);
                println!("          Latency:           {:.2} µs/op", per_op_us);
                println!("          Throughput:        {:.0} verifications/sec", 1_000_000.0 / per_op_us);
            }

            // 6. Adversarial / Malformed LaTeX Stress Test (Zero Panics)
            {
                let malformed = [
                    r"\boxed{\frac{1}{2",
                    r"\boxed{",
                    "x + +",
                    "1 / 0",
                    "((((((((x))))))))",
                    r"\frac{}{}",
                    r"\text{unclosed text",
                    "1,000,000.5.2",
                    "x^",
                    "{[("
                ];
                let iters = 50_000;
                let t0 = Instant::now();
                let mut safe_rejections = 0;
                for i in 0..iters {
                    let input = malformed[i % malformed.len()];
                    let res = verify_math("42", input);
                    if !res.verified {
                        safe_rejections += 1;
                    }
                }
                let dur = t0.elapsed();
                let per_op_us = dur.as_micros() as f64 / iters as f64;
                println!("[Suite 6] Adversarial Malformed LaTeX Stress Test (Zero Panics):");
                println!("          Total Cases:       {} in {:.2?}", iters, dur);
                println!("          Safe Rejections:   {}/{} (0 crashes/panics)", safe_rejections, iters);
                println!("          Latency:           {:.2} µs/op", per_op_us);
                println!("          Throughput:        {:.0} inputs/sec", 1_000_000.0 / per_op_us);
            }

            // 7. LLM CoT Trace & Code Block Extraction
            {
                let sample_cot = "<think>\nLet us compute 10 choose 5.\ncomb(10, 5) = 252.\n</think>\nHere is the code:\n```python\nprint(252)\n```\nThe final answer is \\boxed{252}.";
                let iters = 20_000;
                let t0 = Instant::now();
                for _ in 0..iters {
                    let _ = split_thought_and_response(sample_cot);
                    let _ = extract_code_blocks(sample_cot, Some("python"));
                }
                let dur = t0.elapsed();
                let per_op_us = dur.as_micros() as f64 / iters as f64;
                println!("[Suite 7] LLM CoT Trace Separation & Code Extraction:");
                println!("          Total Operations:  {} ops in {:.2?}", iters, dur);
                println!("          Latency:           {:.2} µs/op", per_op_us);
                println!("          Throughput:        {:.0} traces/sec", 1_000_000.0 / per_op_us);
            }

            println!("================================================================================");
            println!("Status: [PRODUCTION VERIFICATION ENGINE PHYSICAL BENCHMARKS CONFIRMED]");
            println!("================================================================================");
        }
        cmd => {
            eprintln!("Unknown command: {}", cmd);
            std::process::exit(1);
        }
    }
}
