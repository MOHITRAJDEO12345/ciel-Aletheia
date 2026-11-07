use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Zero, One, Signed};
use std::collections::BTreeMap;

/// Canonical multi-variable polynomial with exact arbitrary-precision rational coefficients.
/// Monomial is represented as a sorted map of variable name -> power.
/// Example: 3*x^2*y -> Monomial { "x": 2, "y": 1 }, coeff: 3
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Polynomial {
    // Map: Monomial (BTreeMap<var, exponent>) -> Coefficient (BigRational)
    pub terms: BTreeMap<BTreeMap<String, u32>, BigRational>,
}

impl Polynomial {
    pub fn zero() -> Self {
        Self { terms: BTreeMap::new() }
    }

    pub fn from_constant(c: BigRational) -> Self {
        let mut terms = BTreeMap::new();
        if !c.is_zero() {
            terms.insert(BTreeMap::new(), c);
        }
        Self { terms }
    }

    pub fn from_i64(n: i64) -> Self {
        Self::from_constant(BigRational::from_integer(BigInt::from(n)))
    }

    pub fn from_var(name: &str) -> Self {
        let mut mono = BTreeMap::new();
        mono.insert(name.to_string(), 1);
        let mut terms = BTreeMap::new();
        terms.insert(mono, BigRational::one());
        Self { terms }
    }

    pub fn add(&self, other: &Self) -> Self {
        let mut res = self.terms.clone();
        for (mono, coeff) in &other.terms {
            let entry = res.entry(mono.clone()).or_insert_with(BigRational::zero);
            *entry += coeff;
            if entry.is_zero() {
                res.remove(mono);
            }
        }
        Self { terms: res }
    }

    pub fn sub(&self, other: &Self) -> Self {
        let mut res = self.terms.clone();
        for (mono, coeff) in &other.terms {
            let entry = res.entry(mono.clone()).or_insert_with(BigRational::zero);
            *entry -= coeff;
            if entry.is_zero() {
                res.remove(mono);
            }
        }
        Self { terms: res }
    }

    pub fn mul(&self, other: &Self) -> Self {
        let mut res = BTreeMap::new();
        for (m1, c1) in &self.terms {
            for (m2, c2) in &other.terms {
                // Combine monomials
                let mut m_res = m1.clone();
                for (v, p) in m2 {
                    *m_res.entry(v.clone()).or_insert(0) += p;
                }
                let c_res = c1 * c2;
                let entry = res.entry(m_res).or_insert_with(BigRational::zero);
                *entry += c_res;
            }
        }
        // Cleanup zero terms
        res.retain(|_, c| !c.is_zero());
        Self { terms: res }
    }

    pub fn pow(&self, exp: u32) -> Self {
        if exp == 0 {
            return Self::from_i64(1);
        }
        let mut res = self.clone();
        for _ in 1..exp {
            res = res.mul(self);
        }
        res
    }

    pub fn is_equivalent(&self, other: &Self) -> bool {
        self.sub(other).terms.is_empty()
    }

    pub fn to_canonical_string(&self) -> String {
        if self.terms.is_empty() {
            return "0".to_string();
        }

        let mut parts = Vec::new();
        for (mono, coeff) in self.terms.iter().rev() {
            let mut s = String::new();
            if coeff.is_negative() {
                s.push_str("- ");
            } else if !parts.is_empty() {
                s.push_str("+ ");
            }

            let abs_coeff = coeff.abs();
            let is_const = mono.is_empty();

            if is_const || abs_coeff != BigRational::one() {
                s.push_str(&abs_coeff.to_string());
                if !is_const {
                    s.push('*');
                }
            }

            let mut var_parts = Vec::new();
            for (v, p) in mono {
                if *p == 1 {
                    var_parts.push(v.clone());
                } else {
                    var_parts.push(format!("{}^{}", v, p));
                }
            }
            s.push_str(&var_parts.join("*"));
            parts.push(s);
        }
        parts.join(" ")
    }
}

/// Simple Recursive-Descent Expression Parser for Symbolic Arithmetic
pub struct SymbolicParser<'a> {
    tokens: Vec<Token<'a>>,
    pos: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Token<'a> {
    Num(i64),
    Var(&'a str),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
}

impl<'a> SymbolicParser<'a> {
    pub fn tokenize(input: &'a str) -> Result<Vec<Token<'a>>, String> {
        let mut tokens = Vec::new();
        let bytes = input.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            match bytes[i] {
                b' ' | b'\t' | b'\r' | b'\n' => i += 1,
                b'+' => { tokens.push(Token::Plus); i += 1; }
                b'-' => { tokens.push(Token::Minus); i += 1; }
                b'*' => { tokens.push(Token::Star); i += 1; }
                b'/' => { tokens.push(Token::Slash); i += 1; }
                b'^' => { tokens.push(Token::Caret); i += 1; }
                b'(' => { tokens.push(Token::LParen); i += 1; }
                b')' => { tokens.push(Token::RParen); i += 1; }
                b'0'..=b'9' => {
                    let start = i;
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                    let num: i64 = input[start..i].parse().map_err(|e| format!("Invalid number: {}", e))?;
                    tokens.push(Token::Num(num));
                }
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                    let start = i;
                    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                        i += 1;
                    }
                    tokens.push(Token::Var(&input[start..i]));
                }
                c => return Err(format!("Unexpected character: {}", c as char)),
            }
        }
        Ok(tokens)
    }

    pub fn parse(input: &'a str) -> Result<Polynomial, String> {
        let tokens = Self::tokenize(input)?;
        let mut parser = SymbolicParser { tokens, pos: 0 };
        let res = parser.parse_expr()?;
        if parser.pos < parser.tokens.len() {
            return Err("Trailing tokens in expression".to_string());
        }
        Ok(res)
    }

    fn peek(&self) -> Option<&Token<'a>> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token<'a>> {
        let t = self.tokens.get(self.pos);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn parse_expr(&mut self) -> Result<Polynomial, String> {
        let mut lhs = self.parse_term()?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.advance();
                    let rhs = self.parse_term()?;
                    lhs = lhs.add(&rhs);
                }
                Token::Minus => {
                    self.advance();
                    let rhs = self.parse_term()?;
                    lhs = lhs.sub(&rhs);
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_term(&mut self) -> Result<Polynomial, String> {
        let mut lhs = self.parse_power()?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Star => {
                    self.advance();
                    let rhs = self.parse_power()?;
                    lhs = lhs.mul(&rhs);
                }
                Token::Slash => {
                    self.advance();
                    let rhs = self.parse_power()?;
                    // Constant division
                    if rhs.terms.len() == 1 && rhs.terms.contains_key(&BTreeMap::new()) {
                        let c = rhs.terms.get(&BTreeMap::new()).unwrap();
                        let inv = BigRational::one() / c;
                        lhs = lhs.mul(&Polynomial::from_constant(inv));
                    } else {
                        return Err("Polynomial division by non-constant is unsupported in linear canonicalizer".to_string());
                    }
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_power(&mut self) -> Result<Polynomial, String> {
        let base = self.parse_factor()?;
        if let Some(Token::Caret) = self.peek() {
            self.advance();
            if let Some(Token::Num(p)) = self.peek() {
                let exp = *p as u32;
                self.advance();
                return Ok(base.pow(exp));
            } else {
                return Err("Exponent must be an integer literal".to_string());
            }
        }
        Ok(base)
    }

    fn parse_factor(&mut self) -> Result<Polynomial, String> {
        match self.peek() {
            Some(Token::Num(n)) => {
                let val = *n;
                self.advance();
                Ok(Polynomial::from_i64(val))
            }
            Some(Token::Var(name)) => {
                let var_name = *name;
                self.advance();
                if let Some(Token::LParen) = self.peek() {
                    return Err(format!(
                        "Unsupported non-polynomial function '{}()': Symbolic canonicalizer operates on polynomial and rational algebraic expressions",
                        var_name
                    ));
                }
                Ok(Polynomial::from_var(var_name))
            }
            Some(Token::Minus) => {
                self.advance();
                let f = self.parse_factor()?;
                Ok(Polynomial::zero().sub(&f))
            }
            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_expr()?;
                if let Some(Token::RParen) = self.peek() {
                    self.advance();
                    Ok(expr)
                } else {
                    Err("Expected closing parenthesis ')'".to_string())
                }
            }
            tok => Err(format!("Unexpected token in factor: {:?}", tok)),
        }
    }
}

/// Public API: Check if two mathematical expressions are symbolic equivalents
/// E.g., "(x + 1)^2" and "x^2 + 2*x + 1" -> true
pub fn check_mathematical_equivalence(expr_a: &str, expr_b: &str) -> Result<bool, String> {
    if expr_a.trim() == expr_b.trim() {
        return Ok(true);
    }
    let p_a = SymbolicParser::parse(expr_a)?;
    let p_b = SymbolicParser::parse(expr_b)?;
    Ok(p_a.is_equivalent(&p_b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quadratic_expansion() {
        // (x + 1)^2 == x^2 + 2*x + 1
        assert!(check_mathematical_equivalence("(x + 1)^2", "x^2 + 2*x + 1").unwrap());
    }

    #[test]
    fn test_cubic_expansion() {
        // (x + 1)^3 == x^3 + 3*x^2 + 3*x + 1
        assert!(check_mathematical_equivalence("(x + 1)^3", "x^3 + 3*x^2 + 3*x + 1").unwrap());
    }

    #[test]
    fn test_difference_of_squares() {
        // (x - 3)*(x + 3) == x^2 - 9
        assert!(check_mathematical_equivalence("(x - 3)*(x + 3)", "x^2 - 9").unwrap());
    }

    #[test]
    fn test_distributive_law() {
        // 2*(x + 5) - 3*(x - 2) == 16 - x
        assert!(check_mathematical_equivalence("2*(x + 5) - 3*(x - 2)", "16 - x").unwrap());
    }

    #[test]
    fn test_multivariable_equivalence() {
        // (x + y)^2 == x^2 + 2*x*y + y^2
        assert!(check_mathematical_equivalence("(x + y)^2", "x^2 + 2*x*y + y^2").unwrap());
    }

    #[test]
    fn test_non_polynomial_function_handling() {
        // Identical non-polynomial expressions must return true
        assert!(check_mathematical_equivalence("sin(x)", "sin(x)").unwrap());

        // Non-polynomial function call must return graceful error
        let err = check_mathematical_equivalence("sin(x)", "cos(x)").unwrap_err();
        assert!(err.contains("Unsupported non-polynomial function 'sin()'"));
    }
}
