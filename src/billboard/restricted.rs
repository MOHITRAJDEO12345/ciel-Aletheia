use std::fmt;
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EscalationCapability {
    RequiresFilesystem,
    RequiresBash,
    RequiresNetwork,
    RequiresPackages,
    RequiresProcessSpawn,
}

impl EscalationCapability {
    pub fn as_str(&self) -> &'static str {
        match self {
            EscalationCapability::RequiresFilesystem => "REQUIRES_FILESYSTEM",
            EscalationCapability::RequiresBash => "REQUIRES_BASH",
            EscalationCapability::RequiresNetwork => "REQUIRES_NETWORK",
            EscalationCapability::RequiresPackages => "REQUIRES_PACKAGES",
            EscalationCapability::RequiresProcessSpawn => "REQUIRES_PROCESS_SPAWN",
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub enum AletheiaError {
    RestrictedOperation {
        capability: EscalationCapability,
        attempted: String,
        reason: String,
        upgrade_code: String,
        serialized_state: serde_json::Value,
    },
    OutOfFuel {
        consumed: u64,
    },
    ExecutionError(String),
}

impl AletheiaError {
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            AletheiaError::RestrictedOperation {
                capability,
                attempted,
                reason,
                upgrade_code,
                serialized_state,
            } => json!({
                "error": "ESCALATION_REQUIRED",
                "capability": capability.as_str(),
                "attempted": attempted,
                "reason": reason,
                "upgrade_code": upgrade_code,
                "serialized_state": serialized_state,
            }),
            AletheiaError::OutOfFuel { consumed } => json!({
                "error": "OUT_OF_FUEL",
                "consumed": consumed,
            }),
            AletheiaError::ExecutionError(msg) => json!({
                "error": "EXECUTION_ERROR",
                "message": msg,
            }),
        }
    }
}

impl fmt::Display for AletheiaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AletheiaError::RestrictedOperation {
                capability,
                attempted,
                reason,
                upgrade_code,
                serialized_state,
            } => {
                let json_data = json!({
                    "error": "ESCALATION_REQUIRED",
                    "capability": capability.as_str(),
                    "attempted": attempted,
                    "reason": reason,
                    "upgrade_code": upgrade_code,
                    "serialized_state": serialized_state,
                });
                write!(
                    f,
                    r#"
======================================================================
[E0412] ALETHEIA RESTRICTED OPERATION (FREE IN-MEMORY TIER)
======================================================================
  --> Required Capability: {}
  --> Code Attempted:     `{}`
  --> Reason:             {}

Automated Escalation Snippet:
  {}

JSON Metadata for Autonomous Agents:
  {}
======================================================================
"#,
                    capability.as_str(),
                    attempted,
                    reason,
                    upgrade_code,
                    json_data
                )
            }
            AletheiaError::OutOfFuel { consumed } => {
                write!(f, "OutOfFuel: Execution trapped after {} instruction units (<5µs)", consumed)
            }
            AletheiaError::ExecutionError(msg) => write!(f, "AletheiaExecutionError: {}", msg),
        }
    }
}

impl std::error::Error for AletheiaError {}

fn strip_comments_and_strings(code: &str) -> String {
    let mut result = String::with_capacity(code.len());
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_comment = false;
    let chars: Vec<char> = code.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        let c = chars[i];
        if in_comment {
            if c == '\n' {
                in_comment = false;
                result.push('\n');
            }
            i += 1;
            continue;
        }

        if in_single_quote {
            if c == '\\' && i + 1 < n {
                i += 2;
                continue;
            }
            if c == '\'' {
                in_single_quote = false;
            }
            i += 1;
            continue;
        }

        if in_double_quote {
            if c == '\\' && i + 1 < n {
                i += 2;
                continue;
            }
            if c == '"' {
                in_double_quote = false;
            }
            i += 1;
            continue;
        }

        // Check for comment start (# or //)
        if c == '#' || (c == '/' && i + 1 < n && chars[i + 1] == '/') {
            in_comment = true;
            i += 1;
            continue;
        }

        if c == '\'' {
            in_single_quote = true;
            i += 1;
            continue;
        }

        if c == '"' {
            in_double_quote = true;
            i += 1;
            continue;
        }

        result.push(c);
        i += 1;
    }

    result
}

/// Check for forbidden imports, system commands, and syscall requests
pub fn check_prohibited_patterns(
    code: &str,
    current_env: Option<&serde_json::Value>,
) -> Result<(), AletheiaError> {
    let forbidden = [
        ("open(", EscalationCapability::RequiresFilesystem, "Direct host filesystem I/O requires a Thanaton MicroVM"),
        ("readFile", EscalationCapability::RequiresFilesystem, "Filesystem read access requires a Thanaton MicroVM"),
        ("writeFile", EscalationCapability::RequiresFilesystem, "Filesystem write access requires a Thanaton MicroVM"),
        ("npm install", EscalationCapability::RequiresPackages, "Package installation requires a Thanaton MicroVM with full Linux userspace"),
        ("npm i ", EscalationCapability::RequiresPackages, "Package installation requires a Thanaton MicroVM with full Linux userspace"),
        ("pip install", EscalationCapability::RequiresPackages, "Python package installation requires a Thanaton MicroVM"),
        ("yarn add", EscalationCapability::RequiresPackages, "Package installation requires a Thanaton MicroVM"),
        ("import socket", EscalationCapability::RequiresNetwork, "Raw network sockets require Thanaton MicroVM network virtualization"),
        ("from socket import", EscalationCapability::RequiresNetwork, "Network socket modules require Thanaton MicroVM network virtualization"),
        ("import urllib", EscalationCapability::RequiresNetwork, "Network HTTP libraries require Thanaton MicroVM network virtualization"),
        ("requests.", EscalationCapability::RequiresNetwork, "Outbound HTTP requests require Thanaton MicroVM network virtualization"),
        ("import os", EscalationCapability::RequiresProcessSpawn, "Direct OS syscalls are not available in the in-memory verifier"),
        ("from os import", EscalationCapability::RequiresProcessSpawn, "Direct OS syscalls are not available in the in-memory verifier"),
        ("import sys", EscalationCapability::RequiresProcessSpawn, "Process control modules require a Thanaton MicroVM"),
        ("from sys import", EscalationCapability::RequiresProcessSpawn, "Process control modules require a Thanaton MicroVM"),
        ("import subprocess", EscalationCapability::RequiresProcessSpawn, "Process spawning requires a Thanaton MicroVM"),
        ("from subprocess import", EscalationCapability::RequiresProcessSpawn, "Process spawning requires a Thanaton MicroVM"),
        ("import pty", EscalationCapability::RequiresProcessSpawn, "PTY allocation requires a Thanaton MicroVM"),
        ("fork(", EscalationCapability::RequiresProcessSpawn, "Forking OS processes is strictly forbidden in Tier 0"),
        ("__import__", EscalationCapability::RequiresProcessSpawn, "Dynamic module imports require Thanaton MicroVMs"),
        ("importlib", EscalationCapability::RequiresProcessSpawn, "Dynamic module loaders require Thanaton MicroVMs"),
        ("__builtins__", EscalationCapability::RequiresProcessSpawn, "Accessing raw builtins namespace requires Thanaton MicroVMs"),
        ("builtins.", EscalationCapability::RequiresProcessSpawn, "Accessing raw builtins namespace requires Thanaton MicroVMs"),
        ("ctypes", EscalationCapability::RequiresProcessSpawn, "Raw memory and C foreign function loading require Thanaton MicroVMs"),
        ("dlopen", EscalationCapability::RequiresProcessSpawn, "Dynamic library loading requires Thanaton MicroVMs"),
        ("shutil.", EscalationCapability::RequiresFilesystem, "Filesystem hierarchy operations require Thanaton MicroVMs"),
        ("os.system", EscalationCapability::RequiresProcessSpawn, "OS system calls require Thanaton MicroVMs"),
        ("eval(", EscalationCapability::RequiresProcessSpawn, "Unconstrained eval requires Thanaton MicroVM execution"),
        ("exec(", EscalationCapability::RequiresProcessSpawn, "Unconstrained exec requires Thanaton MicroVM execution"),
        ("compile(", EscalationCapability::RequiresProcessSpawn, "Unconstrained bytecode compilation requires Thanaton MicroVMs"),
    ];

    let state = current_env.cloned().unwrap_or(json!({}));
    let sanitized = strip_comments_and_strings(code);

    // Normalize contiguous whitespace so tabs, multi-spaces, and newlines do not evade detection
    let mut normalized = String::with_capacity(sanitized.len());
    let mut last_space = false;
    for c in sanitized.chars() {
        if c.is_whitespace() {
            if !last_space {
                normalized.push(' ');
                last_space = true;
            }
        } else {
            normalized.push(c);
            last_space = false;
        }
    }

    for (pat, cap, reason) in forbidden {
        if normalized.contains(pat) || sanitized.contains(pat) {
            return Err(AletheiaError::RestrictedOperation {
                capability: cap,
                attempted: pat.to_string(),
                reason: reason.to_string(),
                upgrade_code: "sandbox = ananke.upgrade_to_thanaton(session)".to_string(),
                serialized_state: state,
            });
        }
    }
    Ok(())
}
