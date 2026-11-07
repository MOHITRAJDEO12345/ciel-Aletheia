"""
Escalation bridge connecting free Aletheia in-memory sessions to live Thanaton Cloud sandboxes.
"""

import os
import json
import urllib.request
import urllib.error

def escalate_to_thanaton(session_state: dict, api_key: str = None, base_url: str = None):
    """
    Elevate an in-memory session into a running Thanaton Hardware MicroVM,
    persisting session variables into the guest environment.
    """
    key = api_key or os.getenv("ANANKE_API_KEY") or os.getenv("THANATON_API_KEY") or os.getenv("ESCHATON_API_KEY")
    if not key:
        raise ValueError(
            "Ananke/Thanaton API key required for cloud escalation. "
            "Provision a free key via POST https://compiler.theeducode.com/v1/auth/signup "
            "or pass api_key to Verifier(ananke_api_key='...')."
        )

    endpoint = (base_url or os.getenv("THANATON_BASE_URL") or "https://control.theeducode.com").rstrip("/") + "/v1/sandboxes"
    payload = {
        "memory_mb": 256,
        "runtime": "python",
        "initial_env": session_state,
    }
    req = urllib.request.Request(
        endpoint,
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            "X-API-Key": key,
            "Authorization": f"Bearer {key}",
            "User-Agent": "AletheiaEscalationBridge/1.0"
        },
        method="POST"
    )

    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            return {
                "status": "escalated",
                "sandbox_id": data.get("id", data.get("project_id", "unknown")),
                "runtime": "thanaton-microvm",
                "state": data.get("state", "READY"),
                "memory_mb": data.get("memory_mb", 256),
                "variables_restored": len(session_state),
                "endpoint": endpoint,
            }
    except urllib.error.HTTPError as e:
        err_msg = e.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"Thanaton escalation failed (HTTP {e.code}): {err_msg}")
    except Exception as e:
        raise RuntimeError(f"Failed to connect to Thanaton Control Plane: {e}")
