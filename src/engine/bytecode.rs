use crate::engine::fuel::FuelMeter;
use crate::engine::rational::RationalNumber;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    PushInt(i64),
    PushRational(RationalNumber),
    PushStr(String),
    PushBool(bool),
    LoadVar(String),
    StoreVar(String),
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
    Abs,
    Min,
    Max,
    Gcd,
    Lcm,
    Assert,
    Jump(usize),
    JumpIfFalse(usize),
    Comb, // Combinatorics primitive: comb(n, k)
    Return,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum StackVal {
    Int(i64),
    Rational(RationalNumber),
    Str(String),
    Bool(bool),
}

impl StackVal {
    pub fn as_rational(&self) -> Option<RationalNumber> {
        match self {
            StackVal::Int(i) => Some(RationalNumber::from_i64(*i)),
            StackVal::Rational(r) => Some(r.clone()),
            _ => None,
        }
    }

    pub fn to_i64(&self) -> Option<i64> {
        match self {
            StackVal::Int(i) => Some(*i),
            StackVal::Rational(r) => r.to_i64(),
            _ => None,
        }
    }
}

pub struct VirtualMachine {
    pub stack: Vec<StackVal>,
    pub env: HashMap<String, StackVal>,
    pub pc: usize,
}

impl VirtualMachine {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(64),
            env: HashMap::new(),
            pc: 0,
        }
    }

    pub fn execute(&mut self, code: &[Op], fuel: &FuelMeter) -> Result<StackVal, String> {
        self.pc = 0;
        while self.pc < code.len() {
            // Fuel deduction per instruction
            if fuel.consume(1).is_err() {
                return Err(format!("OutOfFuel: Exceeded instruction budget of {} units", fuel.consumed()));
            }

            match &code[self.pc] {
                Op::PushInt(v) => {
                    self.stack.push(StackVal::Int(*v));
                    self.pc += 1;
                }
                Op::PushRational(r) => {
                    self.stack.push(StackVal::Rational(r.clone()));
                    self.pc += 1;
                }
                Op::PushStr(s) => {
                    self.stack.push(StackVal::Str(s.clone()));
                    self.pc += 1;
                }
                Op::PushBool(b) => {
                    self.stack.push(StackVal::Bool(*b));
                    self.pc += 1;
                }
                Op::LoadVar(name) => {
                    let val = self.env.get(name).cloned().ok_or_else(|| format!("Undefined variable: {}", name))?;
                    self.stack.push(val);
                    self.pc += 1;
                }
                Op::StoreVar(name) => {
                    let val = self.stack.pop().ok_or("Stack underflow on store")?;
                    self.env.insert(name.clone(), val);
                    self.pc += 1;
                }
                Op::Add => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    match (a.as_rational(), b.as_rational()) {
                        (Some(ra), Some(rb)) => self.stack.push(StackVal::Rational(ra.add(&rb))),
                        _ => return Err("Invalid operand types for addition".to_string()),
                    }
                    self.pc += 1;
                }
                Op::Sub => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    match (a.as_rational(), b.as_rational()) {
                        (Some(ra), Some(rb)) => self.stack.push(StackVal::Rational(ra.sub(&rb))),
                        _ => return Err("Invalid operand types for subtraction".to_string()),
                    }
                    self.pc += 1;
                }
                Op::Mul => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    match (a.as_rational(), b.as_rational()) {
                        (Some(ra), Some(rb)) => self.stack.push(StackVal::Rational(ra.mul(&rb))),
                        _ => return Err("Invalid operand types for multiplication".to_string()),
                    }
                    self.pc += 1;
                }
                Op::Div => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    match (a.as_rational(), b.as_rational()) {
                        (Some(ra), Some(rb)) => {
                            let res = ra.div(&rb)?;
                            self.stack.push(StackVal::Rational(res));
                        }
                        _ => return Err("Invalid operand types for division".to_string()),
                    }
                    self.pc += 1;
                }
                Op::Eq => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    match (a.as_rational(), b.as_rational()) {
                        (Some(ra), Some(rb)) => self.stack.push(StackVal::Bool(ra == rb)),
                        _ => self.stack.push(StackVal::Bool(a == b)),
                    }
                    self.pc += 1;
                }
                Op::Neq => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    match (a.as_rational(), b.as_rational()) {
                        (Some(ra), Some(rb)) => self.stack.push(StackVal::Bool(ra != rb)),
                        _ => self.stack.push(StackVal::Bool(a != b)),
                    }
                    self.pc += 1;
                }
                Op::Lt => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    match (a.as_rational(), b.as_rational()) {
                        (Some(ra), Some(rb)) => self.stack.push(StackVal::Bool(ra < rb)),
                        _ => return Err("Invalid operand types for comparison".to_string()),
                    }
                    self.pc += 1;
                }
                Op::Gt => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    match (a.as_rational(), b.as_rational()) {
                        (Some(ra), Some(rb)) => self.stack.push(StackVal::Bool(ra > rb)),
                        _ => return Err("Invalid operand types for comparison".to_string()),
                    }
                    self.pc += 1;
                }
                Op::Lte => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    match (a.as_rational(), b.as_rational()) {
                        (Some(ra), Some(rb)) => self.stack.push(StackVal::Bool(ra <= rb)),
                        _ => return Err("Invalid operand types for comparison".to_string()),
                    }
                    self.pc += 1;
                }
                Op::Gte => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    match (a.as_rational(), b.as_rational()) {
                        (Some(ra), Some(rb)) => self.stack.push(StackVal::Bool(ra >= rb)),
                        _ => return Err("Invalid operand types for comparison".to_string()),
                    }
                    self.pc += 1;
                }
                Op::Abs => {
                    let a = self.stack.pop().ok_or("Stack underflow on abs")?;
                    match a.as_rational() {
                        Some(ra) => self.stack.push(StackVal::Rational(ra.abs())),
                        _ => return Err("abs() requires numeric argument".to_string()),
                    }
                    self.pc += 1;
                }
                Op::Min => {
                    let b = self.stack.pop().ok_or("Stack underflow on min")?;
                    let a = self.stack.pop().ok_or("Stack underflow on min")?;
                    match (a.as_rational(), b.as_rational()) {
                        (Some(ra), Some(rb)) => {
                            if ra <= rb {
                                self.stack.push(a);
                            } else {
                                self.stack.push(b);
                            }
                        }
                        _ => return Err("min() requires numeric arguments".to_string()),
                    }
                    self.pc += 1;
                }
                Op::Max => {
                    let b = self.stack.pop().ok_or("Stack underflow on max")?;
                    let a = self.stack.pop().ok_or("Stack underflow on max")?;
                    match (a.as_rational(), b.as_rational()) {
                        (Some(ra), Some(rb)) => {
                            if ra >= rb {
                                self.stack.push(a);
                            } else {
                                self.stack.push(b);
                            }
                        }
                        _ => return Err("max() requires numeric arguments".to_string()),
                    }
                    self.pc += 1;
                }
                Op::Gcd => {
                    let b = self.stack.pop().ok_or("Stack underflow on gcd")?;
                    let a = self.stack.pop().ok_or("Stack underflow on gcd")?;
                    match (a.to_i64(), b.to_i64()) {
                        (Some(x), Some(y)) => {
                            let mut u = x.abs();
                            let mut v = y.abs();
                            while v != 0 {
                                let t = v;
                                v = u % v;
                                u = t;
                            }
                            self.stack.push(StackVal::Int(u));
                        }
                        _ => return Err("gcd() requires integer arguments".to_string()),
                    }
                    self.pc += 1;
                }
                Op::Lcm => {
                    let b = self.stack.pop().ok_or("Stack underflow on lcm")?;
                    let a = self.stack.pop().ok_or("Stack underflow on lcm")?;
                    match (a.to_i64(), b.to_i64()) {
                        (Some(x), Some(y)) => {
                            let u = x.abs();
                            let v = y.abs();
                            if u == 0 || v == 0 {
                                self.stack.push(StackVal::Int(0));
                            } else {
                                let mut a_gcd = u;
                                let mut b_gcd = v;
                                while b_gcd != 0 {
                                    let t = b_gcd;
                                    b_gcd = a_gcd % b_gcd;
                                    a_gcd = t;
                                }
                                let lcm = (u / a_gcd).saturating_mul(v);
                                self.stack.push(StackVal::Int(lcm));
                            }
                        }
                        _ => return Err("lcm() requires integer arguments".to_string()),
                    }
                    self.pc += 1;
                }
                Op::Assert => {
                    let cond = self.stack.pop().ok_or("Stack underflow on assert")?;
                    match cond {
                        StackVal::Bool(true) => {
                            self.stack.push(StackVal::Bool(true));
                            self.pc += 1;
                        }
                        StackVal::Bool(false) => {
                            return Err("AssertionFailed: Condition evaluated to false".to_string());
                        }
                        _ => return Err("Assertion condition must be boolean".to_string()),
                    }
                }
                Op::Comb => {
                    let k = self.stack.pop().ok_or("Stack underflow")?;
                    let n = self.stack.pop().ok_or("Stack underflow")?;
                    match (n.to_i64(), k.to_i64()) {
                        (Some(nv), Some(kv)) if nv >= 0 && kv >= 0 => {
                            let nv_u = nv as u64;
                            let kv_u = kv as u64;
                            let effective_k = if kv_u > nv_u { 0 } else if kv_u > nv_u - kv_u { nv_u - kv_u } else { kv_u };
                            if effective_k > 10_000 {
                                return Err(format!("comb(n, k) operand size exceeded safety limit (k={}, max=10000)", effective_k));
                            }
                            if effective_k > 0 {
                                let log2_n = (64 - nv_u.leading_zeros()).max(1) as u64;
                                let required_fuel = effective_k.saturating_mul(log2_n);
                                fuel.consume(required_fuel).map_err(|_| "Fuel exhausted during comb calculation")?;
                            }
                            self.stack.push(StackVal::Rational(RationalNumber::comb(nv_u, kv_u)));
                        }
                        _ => return Err("comb(n, k) requires non-negative integers".to_string()),
                    }
                    self.pc += 1;
                }
                Op::Jump(target) => {
                    self.pc = *target;
                }
                Op::JumpIfFalse(target) => {
                    let cond = self.stack.pop().ok_or("Stack underflow on jump")?;
                    match cond {
                        StackVal::Bool(false) => self.pc = *target,
                        StackVal::Bool(true) => self.pc += 1,
                        _ => return Err("JumpIfFalse condition must be a boolean".to_string()),
                    }
                }
                Op::Return => {
                    return self.stack.pop().ok_or_else(|| "Stack empty on return".to_string());
                }
            }
        }
        self.stack.pop().ok_or_else(|| "Execution finished with empty stack".to_string())
    }
}
