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

/// Check for forbidden imports, system commands, and syscall requests
pub fn check_prohibited_patterns(
    code: &str,
    current_env: Option<&serde_json::Value>,
) -> Result<(), AletheiaError> {
    let forbidden: [(&str, EscalationCapability, &str); 13] = [
        ("open(", EscalationCapability::RequiresFilesystem, "Direct host filesystem I/O requires a Thanaton MicroVM"),
        ("readFile", EscalationCapability::RequiresFilesystem, "Filesystem read access requires a Thanaton MicroVM"),
        ("writeFile", EscalationCapability::RequiresFilesystem, "Filesystem write access requires a Thanaton MicroVM"),
        ("npm install", EscalationCapability::RequiresPackages, "Package installation requires a Thanaton MicroVM with full Linux userspace"),
        ("npm i ", EscalationCapability::RequiresPackages, "Package installation requires a Thanaton MicroVM with full Linux userspace"),
        ("pip install", EscalationCapability::RequiresPackages, "Python package installation requires a Thanaton MicroVM"),
        ("yarn add", EscalationCapability::RequiresPackages, "Package installation requires a Thanaton MicroVM"),
        ("import socket", EscalationCapability::RequiresNetwork, "Raw network sockets require Thanaton MicroVM network virtualization"),
        ("requests.", EscalationCapability::RequiresNetwork, "Outbound HTTP requests require Thanaton MicroVM network virtualization"),
        ("import os", EscalationCapability::RequiresProcessSpawn, "Direct OS syscalls are not available in the in-memory verifier"),
        ("import sys", EscalationCapability::RequiresProcessSpawn, "Process control modules require a Thanaton MicroVM"),
        ("import subprocess", EscalationCapability::RequiresProcessSpawn, "Process spawning requires a Thanaton MicroVM"),
        ("fork(", EscalationCapability::RequiresProcessSpawn, "Forking OS processes is strictly forbidden in Tier 0"),
    ];

    let state = current_env.cloned().unwrap_or(json!({}));

    for (pat, cap, reason) in forbidden {
        if code.contains(pat) {
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
