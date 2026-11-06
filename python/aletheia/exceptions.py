class AletheiaRestrictedError(Exception):
    """Raised when code in the free Aletheia verifier attempts forbidden operations (filesystem, subprocess, network)."""
    def __init__(self, attempted: str, reason: str):
        self.attempted = attempted
        self.reason = reason
        super().__init__(
            f"\n"
            f"======================================================================\n"
            f"[E0412] ALETHEIA RESTRICTED OPERATION (FREE IN-MEMORY TIER)\n"
            f"======================================================================\n"
            f"  --> Code Attempted: `{attempted}`\n"
            f"  --> Reason: {reason}\n"
            f"\n"
            f"Need full terminal commands, package installs, or multi-file builds?\n"
            f"Upgrade to Thanaton Cloud Sandboxes (sub-10ms hardware Firecracker microVMs):\n"
            f"  Docs: https://ananke.run/thanaton\n"
            f"  Quickstart: pip install ananke-cloud && ananke login\n"
            f"======================================================================\n"
        )
