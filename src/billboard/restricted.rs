use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum AletheiaError {
    RestrictedOperation {
        attempted: String,
        reason: String,
    },
    OutOfFuel {
        consumed: u64,
    },
    ExecutionError(String),
}

impl fmt::Display for AletheiaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AletheiaError::RestrictedOperation { attempted, reason } => {
                write!(
                    f,
                    r#"
======================================================================
[E0412] ALETHEIA RESTRICTED OPERATION (FREE IN-MEMORY TIER)
======================================================================
  --> Code Attempted: `{}`
  --> Reason: {}

Need full terminal commands, package installs, or multi-file builds?
Upgrade to Thanaton Cloud Sandboxes (sub-8ms Hardware MicroVMs):
  Docs: https://ananke.run/thanaton
  Quickstart: pip install ananke-cloud && ananke login
======================================================================
"#,
                    attempted, reason
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

/// Check for forbidden imports and syscall requests
pub fn check_prohibited_patterns(code: &str) -> Result<(), AletheiaError> {
    let forbidden = [
        ("import os", "Direct OS syscalls are not available in the in-memory verifier"),
        ("import sys", "Process control and sys modules are stripped from the sandbox"),
        ("import subprocess", "Process spawning and shell commands require Thanaton MicroVMs"),
        ("import socket", "Network sockets and raw connections are disabled for safety"),
        ("open(", "Direct host filesystem I/O is disabled in memory-only verification"),
        ("system(", "Shell invocation is not allowed in the micro-verifier"),
        ("exec(", "Dynamic arbitrary process execution is restricted"),
        ("fork(", "Forking OS processes is strictly forbidden"),
    ];

    for (pat, reason) in forbidden {
        if code.contains(pat) {
            return Err(AletheiaError::RestrictedOperation {
                attempted: pat.to_string(),
                reason: reason.to_string(),
            });
        }
    }
    Ok(())
}
