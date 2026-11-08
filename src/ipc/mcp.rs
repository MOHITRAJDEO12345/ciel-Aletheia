use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use crate::engine::session::Session;
use crate::engine::symbolic::check_mathematical_equivalence;
use crate::engine::fuzzer::Fuzzer;
use crate::engine::math_verify::verify_math;
use crate::engine::extractor::{extract_code_blocks, split_thought_and_response, extract_lenient_json};

#[derive(Deserialize, Debug)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

/// Run Model Context Protocol (MCP) server over stdin / stdout
pub fn run_mcp_server() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(trimmed) {
            let id = req.id.unwrap_or(Value::Null);
            let resp = handle_mcp_request(&req.method, req.params, id);
            let resp_json = serde_json::to_string(&resp).unwrap();
            writeln!(writer, "{}", resp_json)?;
            writer.flush()?;
        }
        line.clear();
    }
    Ok(())
}

fn handle_mcp_request(method: &str, params: Option<Value>, id: Value) -> JsonRpcResponse {
    match method {
        "initialize" => {
            let result = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "aletheia-verifier",
                    "version": "0.3.0"
                }
            });
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(result),
                error: None,
            }
        }
        "tools/list" => {
            let result = json!({
                "tools": [
                    {
                        "name": "aletheia_verify_math",
                        "description": "Drop-in Math & LaTeX answer verifier for AI reasoning models (<5µs, zero SymPy, zero GIL). Normalizes LaTeX \\boxed{}, fractions, radicals, percentages, thousands commas, and tests exact rational/symbolic equivalence.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "gold": {
                                    "type": "string",
                                    "description": "Ground truth reference answer or expression (e.g. '\\boxed{\\frac{1}{2}}' or '42')"
                                },
                                "pred": {
                                    "type": "string",
                                    "description": "Candidate model prediction or reasoning output (e.g. 'Therefore, the answer is 0.5')"
                                }
                            },
                            "required": ["gold", "pred"]
                        }
                    },
                    {
                        "name": "aletheia_extract_answer",
                        "description": "Extracts the final mathematical answer from free-form LLM Chain-of-Thought (CoT). Scans for \\boxed{...}, GSM8K '#### ...', or 'The answer is ...'.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "text": {
                                    "type": "string",
                                    "description": "Raw model output or reasoning trace"
                                }
                            },
                            "required": ["text"]
                        }
                    },
                    {
                        "name": "aletheia_extract_code",
                        "description": "Extracts fenced code blocks (```python ... ```) or thought traces (<think> ... </think>) from model outputs.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "text": {
                                    "type": "string",
                                    "description": "Raw model completion text"
                                },
                                "language": {
                                    "type": "string",
                                    "description": "Optional language filter (e.g. 'python', 'json', 'bash')"
                                }
                            },
                            "required": ["text"]
                        }
                    },
                    {
                        "name": "aletheia_eval",
                        "description": "Ultra-low-latency in-memory fuel-metered expression evaluation and exact rational/combinatoric math (<1.09µs). Traps runaway loops in <5µs.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "code": {
                                    "type": "string",
                                    "description": "Math expression or algorithmic snippet (e.g. 'comb(100, 50)' or '100 + 200')"
                                },
                                "fuel": {
                                    "type": "integer",
                                    "description": "Instruction fuel limit (default: 500000)"
                                }
                            },
                            "required": ["code"]
                        }
                    },
                    {
                        "name": "aletheia_eval_multi",
                        "description": "Executes multi-statement in-memory test scripts with 'assert' statements, variable assignments, and builtins (abs, min, max, gcd, lcm, comb) in <3µs.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "script": {
                                    "type": "string",
                                    "description": "Script with statements separated by ';' or newlines (e.g. 'a = 10; b = 20; assert a + b == 30; gcd(a, b)')"
                                },
                                "fuel": {
                                    "type": "integer",
                                    "description": "Instruction fuel limit (default: 500000)"
                                }
                            },
                            "required": ["script"]
                        }
                    },
                    {
                        "name": "aletheia_equiv",
                        "description": "Symbolic mathematical equivalence canonicalizer (<10µs). Verifies whether two algebraic expressions are equivalent (e.g. (x+1)^2 == x^2 + 2x + 1) with zero precision loss.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "expr_a": {
                                    "type": "string",
                                    "description": "First algebraic expression (e.g. '(x + 1)^2')"
                                },
                                "expr_b": {
                                    "type": "string",
                                    "description": "Second algebraic expression (e.g. 'x^2 + 2*x + 1')"
                                }
                            },
                            "required": ["expr_a", "expr_b"]
                        }
                    },
                    {
                        "name": "aletheia_fuzz",
                        "description": "Instant micro-fuzzer evaluating boundary test vectors in <300µs against custom expressions or invariant predicates.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "trials": {
                                    "type": "integer",
                                    "description": "Number of boundary test vectors (default: 1000)"
                                },
                                "expression": {
                                    "type": "string",
                                    "description": "Optional invariant expression to test across random inputs (e.g. 'x + 0 == x' or 'x * 2 == x + x')"
                                }
                            }
                        }
                    }
                ]
            });
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(result),
                error: None,
            }
        }
        "tools/call" => {
            let p = params.unwrap_or(Value::Null);
            let tool_name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = p.get("arguments").cloned().unwrap_or(json!({}));

            match tool_name {
                "aletheia_verify_math" => {
                    let gold = args.get("gold").and_then(|v| v.as_str()).unwrap_or("");
                    let pred = args.get("pred").and_then(|v| v.as_str()).unwrap_or("");
                    let res = verify_math(gold, pred);
                    let result = json!({
                        "content": [
                            {
                                "type": "text",
                                "text": format!("Verified: {} (in {}µs)\nGold Extracted: {}\nPred Extracted: {}", 
                                    res.verified, res.duration_us, res.gold_extracted, res.pred_extracted)
                            }
                        ],
                        "details": res
                    });
                    JsonRpcResponse { jsonrpc: "2.0".to_string(), id, result: Some(result), error: None }
                }
                "aletheia_extract_answer" => {
                    let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let ans = crate::engine::math_verify::extract_answer(text);
                    let result = json!({
                        "content": [
                            {
                                "type": "text",
                                "text": format!("Extracted Answer: {}", ans)
                            }
                        ],
                        "answer": ans
                    });
                    JsonRpcResponse { jsonrpc: "2.0".to_string(), id, result: Some(result), error: None }
                }
                "aletheia_extract_code" => {
                    let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let lang = args.get("language").and_then(|v| v.as_str());
                    let blocks = extract_code_blocks(text, lang);
                    let trace = split_thought_and_response(text);
                    let lenient_json = extract_lenient_json(text);
                    let result = json!({
                        "content": [
                            {
                                "type": "text",
                                "text": format!("Extracted {} code blocks", blocks.len())
                            }
                        ],
                        "blocks": blocks,
                        "thought": trace.thought,
                        "has_json": lenient_json.is_some(),
                        "json_payload": lenient_json
                    });
                    JsonRpcResponse { jsonrpc: "2.0".to_string(), id, result: Some(result), error: None }
                }
                "aletheia_eval_multi" => {
                    let script = args.get("script").and_then(|v| v.as_str()).unwrap_or("");
                    let fuel = args.get("fuel").and_then(|v| v.as_u64()).unwrap_or(500_000);
                    let mut s = Session::new(fuel);
                    match s.eval_multi(script) {
                        Ok(val) => {
                            let result = json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Multi-Statement Result: {:?}", val)
                                    }
                                ],
                                "result": format!("{:?}", val),
                                "fuel_remaining": s.fuel
                            });
                            JsonRpcResponse { jsonrpc: "2.0".to_string(), id, result: Some(result), error: None }
                        }
                        Err(e) => {
                            let err_json = json!({ "code": -32000, "message": format!("{}", e) });
                            JsonRpcResponse { jsonrpc: "2.0".to_string(), id, result: None, error: Some(err_json) }
                        }
                    }
                }
                "aletheia_eval" => {
                    let code = args.get("code").and_then(|v| v.as_str()).unwrap_or("");
                    let fuel = args.get("fuel").and_then(|v| v.as_u64()).unwrap_or(500_000);
                    let mut s = Session::new(fuel);
                    match s.eval_with_telemetry(code) {
                        Ok(telem) => {
                            let result = json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Result: {}\nFuel Consumed: {} units\nLatency: {}µs\nVerified: {}", 
                                            telem.result, telem.fuel_consumed, telem.latency_us, telem.verified)
                                    }
                                ],
                                "telemetry": telem
                            });
                            JsonRpcResponse { jsonrpc: "2.0".to_string(), id, result: Some(result), error: None }
                        }
                        Err(e) => {
                            let err_json = match &e {
                                crate::billboard::restricted::AletheiaError::RestrictedOperation {
                                    capability,
                                    attempted,
                                    reason,
                                    upgrade_code,
                                    serialized_state,
                                } => {
                                    json!({
                                        "code": -32001,
                                        "message": format!("EscalationRequired: Operation requires {} - {}", capability.as_str(), reason),
                                        "data": {
                                             "error": "ESCALATION_REQUIRED",
                                             "capability": capability.as_str(),
                                             "attempted": attempted,
                                             "reason": reason,
                                             "upgrade_code": upgrade_code,
                                             "serialized_state": serialized_state,
                                        }
                                    })
                                }
                                _ => json!({ "code": -32000, "message": format!("{}", e) }),
                            };
                            JsonRpcResponse { jsonrpc: "2.0".to_string(), id, result: None, error: Some(err_json) }
                        }
                    }
                }
                "aletheia_equiv" => {
                    let expr_a = args.get("expr_a").and_then(|v| v.as_str()).unwrap_or("");
                    let expr_b = args.get("expr_b").and_then(|v| v.as_str()).unwrap_or("");
                    match check_mathematical_equivalence(expr_a, expr_b) {
                        Ok(is_eq) => {
                            let result = json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Symbolic Equivalence: {}", is_eq)
                                    }
                                ],
                                "is_equivalent": is_eq
                            });
                            JsonRpcResponse { jsonrpc: "2.0".to_string(), id, result: Some(result), error: None }
                        }
                        Err(e) => {
                            let err_json = json!({ "code": -32000, "message": e });
                            JsonRpcResponse { jsonrpc: "2.0".to_string(), id, result: None, error: Some(err_json) }
                        }
                    }
                }
                "aletheia_fuzz" => {
                    let trials = args.get("trials").and_then(|v| v.as_u64()).unwrap_or(1000) as usize;
                    let expr = args.get("expression").and_then(|v| v.as_str());
                    let fres = match expr {
                        Some(e) => Fuzzer::fuzz_expression(e, trials),
                        None => Fuzzer::fuzz_invariant(|x| x.saturating_add(0) == x, trials),
                    };
                    let result = json!({
                        "content": [
                            {
                                "type": "text",
                                "text": format!("Micro-Fuzzer: {} cases verified in {}µs (Passed: {}){}", 
                                    fres.total_cases, fres.duration_us, fres.passed,
                                    fres.counter_example.as_ref().map(|c| format!(" [{}]", c)).unwrap_or_default())
                            }
                        ],
                        "passed": fres.passed,
                        "duration_us": fres.duration_us,
                        "counter_example": fres.counter_example
                    });
                    JsonRpcResponse { jsonrpc: "2.0".to_string(), id, result: Some(result), error: None }
                }
                _ => {
                    let err_json = json!({ "code": -32601, "message": format!("Tool not found: {}", tool_name) });
                    JsonRpcResponse { jsonrpc: "2.0".to_string(), id, result: None, error: Some(err_json) }
                }
            }
        }
        _ => {
            let err_json = json!({ "code": -32601, "message": format!("Method not found: {}", method) });
            JsonRpcResponse { jsonrpc: "2.0".to_string(), id, result: None, error: Some(err_json) }
        }
    }
}
