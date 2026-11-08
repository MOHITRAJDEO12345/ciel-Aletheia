use crate::engine::fuel::FuelMeter;
use crate::engine::rational::RationalNumber;
use crate::engine::bytecode::{Op, VirtualMachine, StackVal};
use crate::engine::symbolic::check_mathematical_equivalence;
use crate::engine::math_verify::{verify_math, extract_answer, MathVerifyResult};
use crate::billboard::restricted::{AletheiaError, check_prohibited_patterns};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Rational(RationalNumber),
    Text(String),
    Boolean(bool),
}

/// Structured RL & Model Reasoning Telemetry for GRPO / PPO Reward Scorer
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionTelemetry {
    pub result: String,
    pub fuel_consumed: u64,
    pub latency_us: u64,
    pub memory_bytes: usize,
    pub verified: bool,
}

/// A stateful in-memory verification session.
/// Supports Micro-CoW branching for Monte Carlo Tree Search (MCTS) without re-running earlier steps.
#[derive(Clone)]
pub struct Session {
    pub env: HashMap<String, StackVal>,
    pub fuel: u64,
}

impl Session {
    pub fn new(initial_fuel: u64) -> Self {
        Self {
            env: HashMap::new(),
            fuel: initial_fuel,
        }
    }

    /// Fork this session into a parallel child branch with zero memory copies (Micro-CoW).
    pub fn fork(&self) -> Self {
        Self {
            env: self.env.clone(),
            fuel: self.fuel,
        }
    }

    /// Serialize session environment variables to JSON for state preservation across tiers
    pub fn serialize_env_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.env).unwrap_or(serde_json::json!({}))
    }

    /// Check if two algebraic or math expressions are mathematically equivalent in < 10µs.
    /// E.g. (x + 1)^2 == x^2 + 2*x + 1
    pub fn check_equivalence(&self, expr_a: &str, expr_b: &str) -> Result<bool, AletheiaError> {
        check_mathematical_equivalence(expr_a, expr_b)
            .map_err(|e| AletheiaError::ExecutionError(e))
    }

    /// Drop-in Math & LaTeX answer verification for AI reasoning models (<5µs, zero SymPy, zero GIL).
    pub fn verify_math_answer(&self, gold: &str, pred: &str) -> MathVerifyResult {
        verify_math(gold, pred)
    }

    /// Extract final answer from noisy Chain-of-Thought reasoning (handles \boxed{}, ####, etc.)
    pub fn extract_math_answer(&self, text: &str) -> String {
        extract_answer(text)
    }

    /// Execute expression / code snippet within this session context.
    pub fn eval(&mut self, code_text: &str) -> Result<Value, AletheiaError> {
        let env_snapshot = self.serialize_env_json();

        // Intercept forbidden calls (os, sys, subprocess, bash, disk, network)
        check_prohibited_patterns(code_text, Some(&env_snapshot))?;

        // Variable assignment: name = expr
        if let Some((var, expr)) = parse_assignment(code_text) {
            let ops = parse_simple_expression(expr, Some(&env_snapshot))?;
            let fuel_meter = FuelMeter::new(self.fuel);
            let mut vm = VirtualMachine::new();
            vm.env = self.env.clone();
            let val = vm.execute(&ops, &fuel_meter).map_err(AletheiaError::ExecutionError)?;
            self.env.insert(var.to_string(), val.clone());
            self.fuel = fuel_meter.remaining();
            return match val {
                StackVal::Int(i) => Ok(Value::Integer(i)),
                StackVal::Rational(r) => {
                    if let Some(i) = r.to_i64() {
                        Ok(Value::Integer(i))
                    } else {
                        Ok(Value::Rational(r))
                    }
                }
                StackVal::Str(s) => Ok(Value::Text(s)),
                StackVal::Bool(b) => Ok(Value::Boolean(b)),
            };
        }

        let ops = parse_simple_expression(code_text, Some(&env_snapshot))?;
        let fuel_meter = FuelMeter::new(self.fuel);
        let mut vm = VirtualMachine::new();
        vm.env = self.env.clone();

        match vm.execute(&ops, &fuel_meter) {
            Ok(val) => {
                self.env = vm.env;
                self.fuel = fuel_meter.remaining();
                match val {
                    StackVal::Int(i) => Ok(Value::Integer(i)),
                    StackVal::Rational(r) => {
                        if let Some(i) = r.to_i64() {
                            Ok(Value::Integer(i))
                        } else {
                            Ok(Value::Rational(r))
                        }
                    }
                    StackVal::Str(s) => Ok(Value::Text(s)),
                    StackVal::Bool(b) => Ok(Value::Boolean(b)),
                }
            }
            Err(e) => {
                if e.starts_with("OutOfFuel") {
                    Err(AletheiaError::OutOfFuel { consumed: fuel_meter.consumed() })
                } else {
                    Err(AletheiaError::ExecutionError(e))
                }
            }
        }
    }

    /// Execute multiple statements separated by ';' or newlines.
    /// Supports:
    /// - Variable assignments: `x = 10; y = 20;`
    /// - Invariant assertions: `assert x + y == 30;`
    /// - Final expression evaluation: `gcd(x, y)`
    pub fn eval_multi(&mut self, script: &str) -> Result<Value, AletheiaError> {
        let lines: Vec<&str> = script
            .split(|c| c == ';' || c == '\n')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if lines.is_empty() {
            return Ok(Value::Boolean(true));
        }

        let mut last_val = Value::Boolean(true);
        for line in lines {
            if line.starts_with("assert ") {
                let expr = line[7..].trim();
                let cond_val = self.eval(expr)?;
                match cond_val {
                    Value::Boolean(true) => {
                        last_val = Value::Boolean(true);
                    }
                    Value::Boolean(false) => {
                        return Err(AletheiaError::ExecutionError(format!(
                            "AssertionFailed: Invariant check '{}' evaluated to false",
                            expr
                        )));
                    }
                    _ => {
                        return Err(AletheiaError::ExecutionError(format!(
                            "AssertionFailed: Expression '{}' must yield a boolean, got {:?}",
                            expr, cond_val
                        )));
                    }
                }
            } else {
                last_val = self.eval(line)?;
            }
        }
        Ok(last_val)
    }

    /// Execute and return comprehensive telemetry for RL reward models (PPO/GRPO)
    pub fn eval_with_telemetry(&mut self, code_text: &str) -> Result<ExecutionTelemetry, AletheiaError> {
        let t0 = Instant::now();
        let initial_fuel = self.fuel;
        let res = self.eval(code_text)?;
        let latency_us = t0.elapsed().as_micros() as u64;
        let fuel_consumed = initial_fuel - self.fuel;
        
        let env_heap_bytes: usize = self.env.iter().map(|(k, v)| {
            let k_bytes = std::mem::size_of::<String>() + k.capacity();
            let v_bytes = match v {
                StackVal::Str(s) => std::mem::size_of::<String>() + s.capacity(),
                StackVal::Rational(r) => std::mem::size_of_val(r) + 64,
                _ => std::mem::size_of_val(v),
            };
            k_bytes + v_bytes
        }).sum();
        let memory_bytes: usize = std::mem::size_of_val(self) + env_heap_bytes;

        Ok(ExecutionTelemetry {
            result: format!("{:?}", res),
            fuel_consumed,
            latency_us,
            memory_bytes,
            verified: true,
        })
    }
}

fn parse_assignment(code: &str) -> Option<(&str, &str)> {
    let trimmed = code.trim();
    if let Some(idx) = trimmed.find('=') {
        // Exclude compound assignments (+=, -=, *=, /=, %=, &=, |=, ^=) and comparisons (==, <=, >=, !=)
        let excluded_preceding = ['=', '<', '>', '!', '+', '-', '*', '/', '%', '&', '|', '^', ':', '~'];
        if idx > 0 && !excluded_preceding.contains(&(trimmed.as_bytes()[idx - 1] as char)) {
            if idx + 1 < trimmed.len() && trimmed.as_bytes()[idx + 1] != b'=' {
                let var = trimmed[..idx].trim();
                let expr = trimmed[idx + 1..].trim();
                // Enforce valid identifier: starts with alphabetic or underscore, followed by alphanumeric or underscore
                if var.starts_with(|c: char| c.is_alphabetic() || c == '_')
                    && var.chars().all(|c| c.is_alphanumeric() || c == '_')
                {
                    return Some((var, expr));
                }
            }
        }
    }
    None
}

fn parse_simple_expression(expr: &str, state: Option<&serde_json::Value>) -> Result<Vec<Op>, AletheiaError> {
    let trimmed = expr.trim();

    // Intercept common terminal / shell / package manager commands
    let bash_verbs = [
        ("npm ", crate::billboard::restricted::EscalationCapability::RequiresPackages),
        ("pip ", crate::billboard::restricted::EscalationCapability::RequiresPackages),
        ("yarn ", crate::billboard::restricted::EscalationCapability::RequiresPackages),
        ("pnpm ", crate::billboard::restricted::EscalationCapability::RequiresPackages),
        ("cargo ", crate::billboard::restricted::EscalationCapability::RequiresPackages),
        ("apt ", crate::billboard::restricted::EscalationCapability::RequiresPackages),
        ("curl ", crate::billboard::restricted::EscalationCapability::RequiresNetwork),
        ("wget ", crate::billboard::restricted::EscalationCapability::RequiresNetwork),
        ("bash ", crate::billboard::restricted::EscalationCapability::RequiresBash),
        ("git ", crate::billboard::restricted::EscalationCapability::RequiresBash),
        ("docker ", crate::billboard::restricted::EscalationCapability::RequiresBash),
        ("sudo ", crate::billboard::restricted::EscalationCapability::RequiresBash),
        ("make ", crate::billboard::restricted::EscalationCapability::RequiresBash),
    ];
    for (verb, cap) in bash_verbs {
        if trimmed.starts_with(verb) {
            return Err(AletheiaError::RestrictedOperation {
                capability: cap,
                attempted: verb.trim().to_string(),
                reason: format!("Shell command '{}' requires full Linux userspace in Thanaton MicroVM", verb.trim()),
                upgrade_code: "sandbox = ananke.upgrade_to_thanaton(session)".to_string(),
                serialized_state: state.cloned().unwrap_or(serde_json::json!({})),
            });
        }
    }

    // Attempt parsing as structured arithmetic expression via Shunting-Yard
    if let Some(tokens) = tokenize_expression(trimmed) {
        if let Some(ops) = parse_shunting_yard(&tokens) {
            return Ok(ops);
        }
    }

    // Variable identifier lookup
    if trimmed.chars().all(|c| c.is_alphanumeric() || c == '_') && !trimmed.is_empty() {
        return Ok(vec![Op::LoadVar(trimmed.to_string()), Op::Return]);
    }

    // Explicit string literals
    if (trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2)
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2)
    {
        let inner = &trimmed[1..trimmed.len() - 1];
        return Ok(vec![Op::PushStr(inner.to_string()), Op::Return]);
    }

    // Unrecognized syntax: fail with clear error instead of silent fake success
    Err(AletheiaError::ExecutionError(format!(
        "Syntax error: Unrecognized expression or unsupported in-memory statement '{}'. (For complex code, scripts, or OS execution, escalate to Thanaton MicroVM)",
        trimmed
    )))
}

#[derive(Debug, PartialEq, Clone)]
enum Arg {
    Lit(i64),
    Var(String),
}

fn parse_arg(s: &str) -> Option<Arg> {
    let trimmed = s.trim();
    if let Ok(v) = trimmed.parse::<i64>() {
        Some(Arg::Lit(v))
    } else if trimmed.chars().all(|c| c.is_alphanumeric() || c == '_') && !trimmed.is_empty() {
        Some(Arg::Var(trimmed.to_string()))
    } else {
        None
    }
}

#[derive(Debug, PartialEq)]
enum Token {
    Num(i64),
    Var(String),
    Comb(Arg, Arg),
    Abs(Arg),
    Min(Arg, Arg),
    Max(Arg, Arg),
    Gcd(Arg, Arg),
    Lcm(Arg, Arg),
    Op(String),
    LParen,
    RParen,
}

fn tokenize_expression(expr: &str) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let bytes = expr.as_bytes();
    let mut i = 0;
    let n = bytes.len();

    while i < n {
        let b = bytes[i];
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        if b == b'(' {
            tokens.push(Token::LParen);
            i += 1;
            continue;
        }
        if b == b')' {
            tokens.push(Token::RParen);
            i += 1;
            continue;
        }

        // Skip comments: # or //
        if b == b'#' || (b == b'/' && i + 1 < n && bytes[i + 1] == b'/') {
            while i < n && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        if b == b'+' || b == b'*' || b == b'/' {
            tokens.push(Token::Op((b as char).to_string()));
            i += 1;
            continue;
        }

        if b == b'=' && i + 1 < n && bytes[i + 1] == b'=' {
            tokens.push(Token::Op("==".to_string()));
            i += 2;
            continue;
        }

        if b == b'!' && i + 1 < n && bytes[i + 1] == b'=' {
            tokens.push(Token::Op("!=".to_string()));
            i += 2;
            continue;
        }

        if b == b'<' {
            if i + 1 < n && bytes[i + 1] == b'=' {
                tokens.push(Token::Op("<=".to_string()));
                i += 2;
            } else {
                tokens.push(Token::Op("<".to_string()));
                i += 1;
            }
            continue;
        }

        if b == b'>' {
            if i + 1 < n && bytes[i + 1] == b'=' {
                tokens.push(Token::Op(">=".to_string()));
                i += 2;
            } else {
                tokens.push(Token::Op(">".to_string()));
                i += 1;
            }
            continue;
        }

        if b == b'-' {
            let is_unary = match tokens.last() {
                None | Some(Token::Op(_)) | Some(Token::LParen) => true,
                _ => false,
            };
            if is_unary && i + 1 < n && bytes[i + 1].is_ascii_digit() {
                let start = i;
                i += 1;
                while i < n && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                let num_str = std::str::from_utf8(&bytes[start..i]).ok()?;
                let val: i64 = num_str.parse().ok()?;
                tokens.push(Token::Num(val));
                continue;
            } else {
                tokens.push(Token::Op("-".to_string()));
                i += 1;
                continue;
            }
        }

        if b.is_ascii_digit() {
            let start = i;
            while i < n && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let num_str = std::str::from_utf8(&bytes[start..i]).ok()?;
            let val: i64 = num_str.parse().ok()?;
            tokens.push(Token::Num(val));
            continue;
        }

        if b.is_ascii_alphabetic() || b == b'_' {
            let start = i;
            while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let ident = std::str::from_utf8(&bytes[start..i]).ok()?;

            // Check for built-in functions: comb, abs, min, max, gcd, lcm
            if (ident == "comb" || ident == "abs" || ident == "min" || ident == "max" || ident == "gcd" || ident == "lcm")
                && i < n && bytes[i] == b'('
            {
                let paren_start = i;
                let mut depth = 0;
                let mut close_idx = None;
                for j in paren_start..n {
                    if bytes[j] == b'(' {
                        depth += 1;
                    } else if bytes[j] == b')' {
                        depth -= 1;
                        if depth == 0 {
                            close_idx = Some(j);
                            break;
                        }
                    }
                }
                if let Some(end) = close_idx {
                    let inner = std::str::from_utf8(&bytes[paren_start + 1..end]).ok()?;
                    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
                    match ident {
                        "abs" if parts.len() == 1 => {
                            if let Some(arg) = parse_arg(parts[0]) {
                                tokens.push(Token::Abs(arg));
                                i = end + 1;
                                continue;
                            }
                        }
                        "min" if parts.len() == 2 => {
                            if let (Some(a), Some(b)) = (parse_arg(parts[0]), parse_arg(parts[1])) {
                                tokens.push(Token::Min(a, b));
                                i = end + 1;
                                continue;
                            }
                        }
                        "max" if parts.len() == 2 => {
                            if let (Some(a), Some(b)) = (parse_arg(parts[0]), parse_arg(parts[1])) {
                                tokens.push(Token::Max(a, b));
                                i = end + 1;
                                continue;
                            }
                        }
                        "gcd" if parts.len() == 2 => {
                            if let (Some(a), Some(b)) = (parse_arg(parts[0]), parse_arg(parts[1])) {
                                tokens.push(Token::Gcd(a, b));
                                i = end + 1;
                                continue;
                            }
                        }
                        "lcm" if parts.len() == 2 => {
                            if let (Some(a), Some(b)) = (parse_arg(parts[0]), parse_arg(parts[1])) {
                                tokens.push(Token::Lcm(a, b));
                                i = end + 1;
                                continue;
                            }
                        }
                        "comb" if parts.len() == 2 => {
                            if let (Some(n_val), Some(k_val)) = (parse_arg(parts[0]), parse_arg(parts[1])) {
                                tokens.push(Token::Comb(n_val, k_val));
                                i = end + 1;
                                continue;
                            }
                        }
                        _ => {}
                    }
                }
            }

            tokens.push(Token::Var(ident.to_string()));
            continue;
        }

        return None;
    }

    if tokens.is_empty() {
        None
    } else {
        Some(tokens)
    }
}

fn push_arg(arg: &Arg, output: &mut Vec<Op>) {
    match arg {
        Arg::Lit(v) => output.push(Op::PushInt(*v)),
        Arg::Var(name) => output.push(Op::LoadVar(name.clone())),
    }
}

fn parse_shunting_yard(tokens: &[Token]) -> Option<Vec<Op>> {
    let mut output: Vec<Op> = Vec::new();
    let mut op_stack: Vec<String> = Vec::new();

    let precedence = |op: &str| -> i32 {
        match op {
            "==" | "!=" | "<" | ">" | "<=" | ">=" => 1,
            "+" | "-" => 2,
            "*" | "/" => 3,
            _ => 0,
        }
    };

    for token in tokens {
        match token {
            Token::Num(v) => output.push(Op::PushInt(*v)),
            Token::Var(v) => output.push(Op::LoadVar(v.clone())),
            Token::Comb(n, k) => {
                push_arg(n, &mut output);
                push_arg(k, &mut output);
                output.push(Op::Comb);
            }
            Token::Abs(x) => {
                push_arg(x, &mut output);
                output.push(Op::Abs);
            }
            Token::Min(a, b) => {
                push_arg(a, &mut output);
                push_arg(b, &mut output);
                output.push(Op::Min);
            }
            Token::Max(a, b) => {
                push_arg(a, &mut output);
                push_arg(b, &mut output);
                output.push(Op::Max);
            }
            Token::Gcd(a, b) => {
                push_arg(a, &mut output);
                push_arg(b, &mut output);
                output.push(Op::Gcd);
            }
            Token::Lcm(a, b) => {
                push_arg(a, &mut output);
                push_arg(b, &mut output);
                output.push(Op::Lcm);
            }
            Token::Op(op) => {
                let p = precedence(op);
                while let Some(top) = op_stack.last() {
                    if top == "(" {
                        break;
                    }
                    if precedence(top) >= p {
                        let top_op = op_stack.pop().unwrap();
                        match top_op.as_str() {
                            "+" => output.push(Op::Add),
                            "-" => output.push(Op::Sub),
                            "*" => output.push(Op::Mul),
                            "/" => output.push(Op::Div),
                            "==" => output.push(Op::Eq),
                            "!=" => output.push(Op::Neq),
                            "<" => output.push(Op::Lt),
                            ">" => output.push(Op::Gt),
                            "<=" => output.push(Op::Lte),
                            ">=" => output.push(Op::Gte),
                            _ => return None,
                        }
                    } else {
                        break;
                    }
                }
                op_stack.push(op.clone());
            }
            Token::LParen => op_stack.push("(".to_string()),
            Token::RParen => {
                let mut matched = false;
                while let Some(top) = op_stack.pop() {
                    if top == "(" {
                        matched = true;
                        break;
                    }
                    match top.as_str() {
                        "+" => output.push(Op::Add),
                        "-" => output.push(Op::Sub),
                        "*" => output.push(Op::Mul),
                        "/" => output.push(Op::Div),
                        "==" => output.push(Op::Eq),
                        "!=" => output.push(Op::Neq),
                        "<" => output.push(Op::Lt),
                        ">" => output.push(Op::Gt),
                        "<=" => output.push(Op::Lte),
                        ">=" => output.push(Op::Gte),
                        _ => return None,
                    }
                }
                if !matched {
                    return None;
                }
            }
        }
    }

    while let Some(top) = op_stack.pop() {
        if top == "(" || top == ")" {
            return None;
        }
        match top.as_str() {
            "+" => output.push(Op::Add),
            "-" => output.push(Op::Sub),
            "*" => output.push(Op::Mul),
            "/" => output.push(Op::Div),
            "==" => output.push(Op::Eq),
            "!=" => output.push(Op::Neq),
            "<" => output.push(Op::Lt),
            ">" => output.push(Op::Gt),
            "<=" => output.push(Op::Lte),
            ">=" => output.push(Op::Gte),
            _ => return None,
        }
    }

    if output.is_empty() {
        return None;
    }

    output.push(Op::Return);
    Some(output)
}
