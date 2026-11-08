use crate::engine::rational::RationalNumber;
use crate::engine::symbolic::check_mathematical_equivalence;
use std::time::Instant;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MathVerifyResult {
    pub verified: bool,
    pub gold_extracted: String,
    pub pred_extracted: String,
    pub gold_normalized: String,
    pub pred_normalized: String,
    pub duration_us: u64,
}

/// Extract content inside \boxed{...} with balanced brace tracking.
/// Handles arbitrarily nested LaTeX braces like \boxed{\frac{1}{2}} or \boxed{\sqrt{\frac{x}{y}}}.
pub fn extract_boxed(text: &str) -> Option<String> {
    let boxed_tag = "\\boxed{";
    if let Some(start_idx) = text.rfind(boxed_tag) {
        let content_start = start_idx + boxed_tag.len();
        let bytes = text.as_bytes();
        let mut depth = 1;
        let mut idx = content_start;

        while idx < bytes.len() {
            match bytes[idx] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(text[content_start..idx].trim().to_string());
                    }
                }
                _ => {}
            }
            idx += 1;
        }
    }
    None
}

/// Extract GSM8K style answer formatted as "#### <answer>"
pub fn extract_gsm8k(text: &str) -> Option<String> {
    if let Some(pos) = text.rfind("####") {
        let after = &text[pos + 4..].trim();
        let first_line = after.lines().next().unwrap_or(after).trim();
        if !first_line.is_empty() {
            return Some(first_line.to_string());
        }
    }
    None
}

/// General answer extractor for LLM reasoning traces.
/// Prioritizes:
/// 1. \boxed{...} (MATH / AIME / Olympiad standard)
/// 2. #### ... (GSM8K standard)
/// 3. "The final answer is ..." or "The answer is ..."
/// 4. Last non-empty line of the text
pub fn extract_answer(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(boxed) = extract_boxed(trimmed) {
        return boxed;
    }
    if let Some(gsm8k) = extract_gsm8k(trimmed) {
        return gsm8k;
    }

    // Check for "the answer is" or "the final answer is" (case-insensitive)
    let lower = trimmed.to_lowercase();
    let markers = ["the final answer is", "the answer is", "answer:", "final answer:"];
    for marker in markers {
        if let Some(pos) = lower.rfind(marker) {
            let after = &trimmed[pos + marker.len()..].trim();
            let first_line = after.lines().next().unwrap_or(after).trim();
            let clean = first_line.trim_end_matches('.').trim();
            if !clean.is_empty() {
                // If it contains a boxed inside the answer line, extract that
                if let Some(boxed) = extract_boxed(clean) {
                    return boxed;
                }
                return clean.to_string();
            }
        }
    }

    // Fallback: Last non-empty line
    trimmed.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or(trimmed).trim().to_string()
}

/// Normalizes LaTeX math strings into canonical mathematical representations.
/// Strips formatting, converts LaTeX fractions/radicals/operators, removes currency signs and thousand separators.
pub fn normalize_math(expr: &str) -> String {
    let mut s = expr.trim().to_string();

    // 1. Remove enclosing display math delimiters: $...$, $$...$$, \[...\], \(...\)
    if s.starts_with("$$") && s.ends_with("$$") && s.len() >= 4 {
        s = s[2..s.len() - 2].trim().to_string();
    } else if s.starts_with('$') && s.ends_with('$') && s.len() >= 2 {
        s = s[1..s.len() - 1].trim().to_string();
    } else if s.starts_with("\\[") && s.ends_with("\\]") && s.len() >= 4 {
        s = s[2..s.len() - 2].trim().to_string();
    } else if s.starts_with("\\(") && s.ends_with("\\)") && s.len() >= 4 {
        s = s[2..s.len() - 2].trim().to_string();
    }

    // 2. Strip LaTeX text and formatting wrappers: \text{...}, \mathrm{...}, \mathbf{...}, \displaystyle
    s = s.replace("\\displaystyle", "");
    s = strip_latex_wrapper(&s, "\\text{");
    s = strip_latex_wrapper(&s, "\\mathrm{");
    s = strip_latex_wrapper(&s, "\\mathbf{");
    s = strip_latex_wrapper(&s, "\\mathit{");
    s = strip_latex_wrapper(&s, "\\operatorname{");

    // 3. Strip units and currency symbols
    s = s.replace("\\$", "");
    s = s.replace('$', "");
    s = s.replace('€', "");
    s = s.replace('£', "");
    s = s.replace('¥', "");

    // 4. Remove thousand separator commas in numbers (e.g., 1,000 -> 1000)
    s = remove_thousand_commas(&s);

    // 5. Handle percentages: 25\% or 25% -> 25/100
    s = s.replace("\\%", "%");
    if s.ends_with('%') {
        let num_part = s[..s.len() - 1].trim();
        s = format!("({})/100", num_part);
    }

    // 6. Convert LaTeX operators to standard math operators
    s = s.replace("\\times", "*");
    s = s.replace("\\cdot", "*");
    s = s.replace("\\div", "/");
    s = s.replace("\\pm", "+-");
    s = s.replace("\\le", "<=");
    s = s.replace("\\ge", ">=");
    s = s.replace("\\leq", "<=");
    s = s.replace("\\geq", ">=");
    s = s.replace("\\neq", "!=");

    // 7. Expand LaTeX fractions: \frac{a}{b}, \dfrac{a}{b}, \tfrac{a}{b} -> (a)/(b)
    s = expand_latex_fractions(&s);

    // 8. Expand square roots: \sqrt{x} -> (x)^(1/2)
    s = expand_latex_sqrt(&s);

    // 9. Insert implicit multiplication: e.g. 2x -> 2*x, 3(x) -> 3*(x), (a)(b) -> (a)*(b)
    s = insert_implicit_multiplication(&s);

    // 10. Clean up spaces and trailing punctuation
    s = s.trim_end_matches('.').trim().to_string();
    s
}

/// Verifies whether a predicted answer matches the gold reference answer.
/// Handles:
/// - CoT text extraction (\boxed{}, ####, etc.)
/// - Formatting differences (LaTeX fractions, decimals, percentages)
/// - Exact rational arithmetic (e.g. 1/2 == 0.5 == 2/4 == 50%)
/// - Symbolic equivalence (e.g. (x+1)^2 == x^2 + 2x + 1)
pub fn verify_math(gold: &str, pred: &str) -> MathVerifyResult {
    let t0 = Instant::now();

    let gold_ext = extract_answer(gold);
    let pred_ext = extract_answer(pred);

    let gold_norm = normalize_math(&gold_ext);
    let pred_norm = normalize_math(&pred_ext);

    let mut verified = false;

    // 1. Direct case-insensitive match
    if gold_norm.eq_ignore_ascii_case(&pred_norm) {
        verified = true;
    }

    // 2. Exact Rational Arithmetic match (e.g., "1/2" vs "0.5")
    if !verified {
        if let (Some(r_gold), Some(r_pred)) = (parse_number_or_fraction(&gold_norm), parse_number_or_fraction(&pred_norm)) {
            if r_gold == r_pred {
                verified = true;
            }
        }
    }

    // 3. Floating point approximate match for decimal representations (within 1e-6 epsilon)
    if !verified {
        if let (Ok(f_gold), Ok(f_pred)) = (gold_norm.parse::<f64>(), pred_norm.parse::<f64>()) {
            if (f_gold - f_pred).abs() < 1e-6 {
                verified = true;
            }
        }
    }

    // 4. Symbolic equivalence check (polynomial / algebraic identity)
    if !verified {
        if let Ok(is_eq) = check_mathematical_equivalence(&gold_norm, &pred_norm) {
            if is_eq {
                verified = true;
            }
        }
    }

    MathVerifyResult {
        verified,
        gold_extracted: gold_ext,
        pred_extracted: pred_ext,
        gold_normalized: gold_norm,
        pred_normalized: pred_norm,
        duration_us: t0.elapsed().as_micros() as u64,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn insert_implicit_multiplication(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    let chars: Vec<char> = s.chars().collect();
    for i in 0..chars.len() {
        out.push(chars[i]);
        if i + 1 < chars.len() {
            let c1 = chars[i];
            let c2 = chars[i + 1];
            let is_implicit = (c1.is_ascii_digit() && (c2.is_ascii_alphabetic() || c2 == '('))
                || (c1 == ')' && (c2.is_ascii_alphabetic() || c2.is_ascii_digit() || c2 == '('));
            if is_implicit {
                out.push('*');
            }
        }
    }
    out
}

fn strip_latex_wrapper(input: &str, prefix: &str) -> String {
    let mut result = input.to_string();
    while let Some(start) = result.find(prefix) {
        let content_start = start + prefix.len();
        let bytes = result.as_bytes();
        let mut depth = 1;
        let mut end_opt = None;
        for i in content_start..bytes.len() {
            if bytes[i] == b'{' {
                depth += 1;
            } else if bytes[i] == b'}' {
                depth -= 1;
                if depth == 0 {
                    end_opt = Some(i);
                    break;
                }
            }
        }
        if let Some(end) = end_opt {
            let inner = &result[content_start..end];
            result = format!("{}{}{}", &result[..start], inner, &result[end + 1..]);
        } else {
            break;
        }
    }
    result
}

fn remove_thousand_commas(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b',' {
            let prev_is_digit = i > 0 && bytes[i - 1].is_ascii_digit();
            let next_is_digit = i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit();
            if prev_is_digit && next_is_digit {
                continue; // Skip comma in 1,000
            }
        }
        out.push(b as char);
    }
    out
}

fn expand_latex_fractions(s: &str) -> String {
    let prefixes = ["\\frac{", "\\dfrac{", "\\tfrac{"];
    let mut result = s.to_string();

    'outer: loop {
        for prefix in &prefixes {
            if let Some(start) = result.find(prefix) {
                // Find numerator
                let num_start = start + prefix.len();
                if let Some((num, num_end)) = extract_balanced_braces(&result, num_start) {
                    // Check if followed by { (denominator)
                    if num_end < result.len() && result.as_bytes()[num_end] == b'{' {
                        let den_start = num_end + 1;
                        if let Some((den, den_end)) = extract_balanced_braces(&result, den_start) {
                            let replacement = format!("(({})/({}))", num, den);
                            result = format!("{}{}{}", &result[..start], replacement, &result[den_end..]);
                            continue 'outer;
                        }
                    }
                }
            }
        }
        break;
    }
    result
}

fn expand_latex_sqrt(s: &str) -> String {
    let prefix = "\\sqrt{";
    let mut result = s.to_string();

    while let Some(start) = result.find(prefix) {
        let content_start = start + prefix.len();
        if let Some((inner, end)) = extract_balanced_braces(&result, content_start) {
            let replacement = format!("(({})^(1/2))", inner);
            result = format!("{}{}{}", &result[..start], replacement, &result[end..]);
        } else {
            break;
        }
    }
    result
}

fn extract_balanced_braces(s: &str, content_start: usize) -> Option<(&str, usize)> {
    let bytes = s.as_bytes();
    let mut depth = 1;
    for i in content_start..bytes.len() {
        if bytes[i] == b'{' {
            depth += 1;
        } else if bytes[i] == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some((&s[content_start..i], i + 1));
            }
        }
    }
    None
}

/// Parses a string into a RationalNumber if it is an integer, decimal, or simple fraction (a/b).
fn parse_number_or_fraction(s: &str) -> Option<RationalNumber> {
    let trimmed = s.trim().trim_matches('(').trim_matches(')').trim();

    // Check for fraction: "a/b"
    if let Some(slash_idx) = trimmed.find('/') {
        let num_str = trimmed[..slash_idx].trim().trim_matches('(').trim_matches(')');
        let den_str = trimmed[slash_idx + 1..].trim().trim_matches('(').trim_matches(')');

        if let (Ok(num), Ok(den)) = (num_str.parse::<i64>(), den_str.parse::<i64>()) {
            if den != 0 {
                return Some(RationalNumber::new(num, den));
            }
        }
    }

    // Check for integer
    if let Ok(i) = trimmed.parse::<i64>() {
        return Some(RationalNumber::from_i64(i));
    }

    // Check for decimal: "3.14" -> 314/100
    if let Some(dot_idx) = trimmed.find('.') {
        let int_part = &trimmed[..dot_idx];
        let frac_part = &trimmed[dot_idx + 1..];

        if frac_part.chars().all(|c| c.is_ascii_digit()) && !frac_part.is_empty() {
            let full_str = format!("{}{}", int_part, frac_part);
            if let Ok(num) = full_str.parse::<i64>() {
                let den = 10_i64.pow(frac_part.len() as u32);
                return Some(RationalNumber::new(num, den));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_boxed_nested_braces() {
        let text = "Therefore, the final answer is \\boxed{\\frac{1}{2}}.";
        assert_eq!(extract_boxed(text), Some("\\frac{1}{2}".to_string()));

        let text_complex = "Solution: \\boxed{\\sqrt{\\frac{3}{4}}}. Done.";
        assert_eq!(extract_boxed(text_complex), Some("\\sqrt{\\frac{3}{4}}".to_string()));
    }

    #[test]
    fn test_extract_gsm8k() {
        let text = "Step 1: 5 * 2 = 10\nStep 2: 10 + 32 = 42\n#### 42";
        assert_eq!(extract_gsm8k(text), Some("42".to_string()));
    }

    #[test]
    fn test_normalize_latex_math() {
        assert_eq!(normalize_math("\\$1,000"), "1000");
        assert_eq!(normalize_math("\\frac{1}{2}"), "((1)/(2))");
        assert_eq!(normalize_math("50\\%"), "(50)/100");
        assert_eq!(normalize_math("3 \\times 4"), "3 * 4");
    }

    #[test]
    fn test_verify_math_equivalence() {
        // Exact fraction vs decimal
        let res = verify_math("\\boxed{\\frac{1}{2}}", "0.5");
        assert!(res.verified);

        // GSM8K integer
        let res2 = verify_math("#### 42", "The answer is 42.");
        assert!(res2.verified);

        // Percentage vs fraction
        let res3 = verify_math("50%", "1/2");
        assert!(res3.verified);

        // Thousands separator
        let res4 = verify_math("1000", "1,000");
        assert!(res4.verified);

        // Symbolic equivalence
        let res5 = verify_math("\\boxed{(x + 1)^2}", "x^2 + 2x + 1");
        assert!(res5.verified);
    }
}
