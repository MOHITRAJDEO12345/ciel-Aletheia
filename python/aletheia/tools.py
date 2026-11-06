"""
OpenAI, Anthropic, and Gemini Function Calling Schema Definitions for Aletheia.
"""

def get_openai_tools():
    """Returns OpenAI / Anthropic / Gemini compatible tool definitions."""
    return [
        {
            "type": "function",
            "function": {
                "name": "aletheia_eval",
                "description": "Ultra-low-latency in-memory fuel-metered math and algorithmic evaluation (<1.09µs). Traps runaway loops in <5µs with exact rational arithmetic.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "Mathematical expression or arithmetic algorithm to verify (e.g. 'comb(100, 50)', '100 + 200')"
                        },
                        "fuel": {
                            "type": "integer",
                            "description": "Instruction fuel limit (default: 500,000)"
                        }
                    },
                    "required": ["code"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "aletheia_equiv",
                "description": "Symbolic mathematical equivalence canonicalizer (<10µs). Verifies whether two algebraic expressions are equivalent (e.g. (x+1)^2 == x^2 + 2x + 1) without precision drift.",
                "parameters": {
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
            }
        },
        {
            "type": "function",
            "function": {
                "name": "aletheia_fuzz",
                "description": "Instant micro-fuzzer evaluating 1,000 numerical and logical boundary test vectors in <300µs.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "trials": {
                            "type": "integer",
                            "description": "Number of boundary test vectors (default: 1000)"
                        }
                    }
                }
            }
        }
    ]

