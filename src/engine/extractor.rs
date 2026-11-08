use serde_json::Value;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractedCodeBlock {
    pub language: String,
    pub code: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReasoningTrace {
    pub thought: Option<String>,
    pub response: String,
}

/// Splits a DeepSeek-R1 / reasoning model completion into its `<think>...</think>` section
/// and its final response section.
pub fn split_thought_and_response(text: &str) -> ReasoningTrace {
    let think_start = "<think>";
    let think_end = "</think>";

    if let Some(start_pos) = text.find(think_start) {
        let content_start = start_pos + think_start.len();
        if let Some(end_offset) = text[content_start..].find(think_end) {
            let thought_content = text[content_start..content_start + end_offset].trim().to_string();
            let response_content = text[content_start + end_offset + think_end.len()..].trim().to_string();
            return ReasoningTrace {
                thought: Some(thought_content),
                response: response_content,
            };
        } else {
            // Unclosed <think> tag (model ran out of tokens while thinking)
            let thought_content = text[content_start..].trim().to_string();
            return ReasoningTrace {
                thought: Some(thought_content),
                response: String::new(),
            };
        }
    }

    ReasoningTrace {
        thought: None,
        response: text.trim().to_string(),
    }
}

/// Extracts all fenced code blocks from markdown (e.g. ```python ... ```).
/// Can filter by specific programming language (e.g. Some("python")).
pub fn extract_code_blocks(text: &str, lang_filter: Option<&str>) -> Vec<ExtractedCodeBlock> {
    let mut blocks = Vec::new();
    let fence = "```";
    let mut search_idx = 0;

    while let Some(start) = text[search_idx..].find(fence) {
        let block_start = search_idx + start + fence.len();
        // Read language identifier up to newline
        if let Some(newline_rel) = text[block_start..].find('\n') {
            let lang = text[block_start..block_start + newline_rel].trim().to_lowercase();
            let code_start = block_start + newline_rel + 1;

            if let Some(end_rel) = text[code_start..].find(fence) {
                let code_content = text[code_start..code_start + end_rel].trim_end().to_string();
                let matches_lang = match lang_filter {
                    Some(wanted) => lang.eq_ignore_ascii_case(wanted),
                    None => true,
                };
                if matches_lang {
                    blocks.push(ExtractedCodeBlock {
                        language: lang,
                        code: code_content,
                    });
                }
                search_idx = code_start + end_rel + fence.len();
                continue;
            }
        }
        break;
    }

    blocks
}

/// Lenient JSON extraction for LLM tool calls and structured outputs.
/// Handles:
/// 1. Markdown code fencing (` ```json ... ``` `)
/// 2. Raw JSON objects `{ ... }` or arrays `[ ... ]`
/// 3. Trailing commas before `}` or `]`
pub fn extract_lenient_json(text: &str) -> Option<Value> {
    let trimmed = text.trim();

    // 1. Direct parse attempt
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        return Some(v);
    }

    // 2. Check for markdown fenced JSON
    let blocks = extract_code_blocks(trimmed, Some("json"));
    if let Some(first) = blocks.first() {
        if let Ok(v) = serde_json::from_str::<Value>(&first.code) {
            return Some(v);
        }
        if let Some(cleaned) = clean_trailing_commas(&first.code) {
            if let Ok(v) = serde_json::from_str::<Value>(&cleaned) {
                return Some(v);
            }
        }
    }

    // 3. Scan for first '{' and matching balanced '}'
    if let Some(start) = trimmed.find('{') {
        let bytes = trimmed.as_bytes();
        let mut depth = 0;
        let mut end_opt = None;
        for i in start..bytes.len() {
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
            let candidate = &trimmed[start..=end];
            if let Ok(v) = serde_json::from_str::<Value>(candidate) {
                return Some(v);
            }
            if let Some(cleaned) = clean_trailing_commas(candidate) {
                if let Ok(v) = serde_json::from_str::<Value>(&cleaned) {
                    return Some(v);
                }
            }
        }
    }

    None
}

/// Cleans common JSON syntax issues from LLMs, such as trailing commas before closing braces/brackets
fn clean_trailing_commas(json_str: &str) -> Option<String> {
    let mut out = String::with_capacity(json_str.len());
    let bytes = json_str.as_bytes();
    let n = bytes.len();
    let mut i = 0;

    while i < n {
        let b = bytes[i];
        if b == b',' {
            // Look ahead skipping whitespace to see if next non-ws char is '}' or ']'
            let mut j = i + 1;
            while j < n && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\r' || bytes[j] == b'\n') {
                j += 1;
            }
            if j < n && (bytes[j] == b'}' || bytes[j] == b']') {
                // Skip the comma!
                i += 1;
                continue;
            }
        }
        out.push(b as char);
        i += 1;
    }

    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_thought_and_response() {
        let text = "<think>\nThinking deeply about the problem...\nLet x = 42\n</think>\nThe answer is 42.";
        let res = split_thought_and_response(text);
        assert_eq!(res.thought, Some("Thinking deeply about the problem...\nLet x = 42".to_string()));
        assert_eq!(res.response, "The answer is 42.");
    }

    #[test]
    fn test_extract_code_blocks() {
        let text = "Here is Python code:\n```python\ndef solve():\n    return 42\n```\nAnd bash:\n```bash\necho hello\n```";
        let py_blocks = extract_code_blocks(text, Some("python"));
        assert_eq!(py_blocks.len(), 1);
        assert_eq!(py_blocks[0].code, "def solve():\n    return 42");

        let all_blocks = extract_code_blocks(text, None);
        assert_eq!(all_blocks.len(), 2);
    }

    #[test]
    fn test_extract_lenient_json() {
        // Trailing comma
        let bad_json = "```json\n{\"name\": \"test\", \"value\": 123, }\n```";
        let parsed = extract_lenient_json(bad_json).expect("Failed to parse lenient json");
        assert_eq!(parsed["name"], "test");
        assert_eq!(parsed["value"], 123);
    }
}

