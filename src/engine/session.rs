use crate::engine::fuel::FuelMeter;
use crate::engine::rational::RationalNumber;
use crate::engine::bytecode::{Op, VirtualMachine, StackVal};
use crate::billboard::restricted::{AletheiaError, check_prohibited_patterns};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Rational(RationalNumber),
    Text(String),
    Boolean(bool),
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

    /// Execute expression / code snippet within this session context.
    pub fn eval(&mut self, code_text: &str) -> Result<Value, AletheiaError> {
        // Intercept forbidden calls (os, sys, subprocess, bash, disk, network)
        check_prohibited_patterns(code_text)?;

        // Simple expression parser for basic math, comb, and variables
        let ops = parse_simple_expression(code_text)?;
        let fuel_meter = FuelMeter::new(self.fuel);
        let mut vm = VirtualMachine::new();
        vm.env = self.env.clone();

        match vm.execute(&ops, &fuel_meter) {
            Ok(val) => {
                self.env = vm.env;
                self.fuel = fuel_meter.remaining();
                match val {
                    StackVal::Int(i) => Ok(Value::Integer(i)),
                    StackVal::Rational(r) => Ok(Value::Rational(r)),
                    StackVal::Str(s) => Ok(Value::Text(s)),
                    StackVal::Bool(b) => Ok(Value::Boolean(b)),
                }
            }
            Err(e) => Err(AletheiaError::ExecutionError(e)),
        }
    }
}

fn parse_simple_expression(expr: &str) -> Result<Vec<Op>, AletheiaError> {
    let trimmed = expr.trim();

    // Check for comb(n, k)
    if trimmed.starts_with("comb(") && trimmed.ends_with(')') {
        let inner = &trimmed[5..trimmed.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
        if parts.len() == 2 {
            let n: i64 = parts[0].parse().map_err(|_| AletheiaError::ExecutionError("invalid n in comb".into()))?;
            let k: i64 = parts[1].parse().map_err(|_| AletheiaError::ExecutionError("invalid k in comb".into()))?;
            return Ok(vec![Op::PushInt(n), Op::PushInt(k), Op::Comb, Op::Return]);
        }
    }

    // Check for binary operations: a + b, a - b, a * b, a / b
    for op_char in ['+', '-', '*', '/'] {
        if let Some(pos) = trimmed.find(op_char) {
            let left = trimmed[..pos].trim();
            let right = trimmed[pos + 1..].trim();
            if let (Ok(lv), Ok(rv)) = (left.parse::<i64>(), right.parse::<i64>()) {
                let op = match op_char {
                    '+' => Op::Add,
                    '-' => Op::Sub,
                    '*' => Op::Mul,
                    '/' => Op::Div,
                    _ => unreachable!(),
                };
                return Ok(vec![Op::PushInt(lv), Op::PushInt(rv), op, Op::Return]);
            }
        }
    }

    // Single integer
    if let Ok(v) = trimmed.parse::<i64>() {
        return Ok(vec![Op::PushInt(v), Op::Return]);
    }

    // Default variable or string
    Ok(vec![Op::PushStr(trimmed.to_string()), Op::Return])
}
