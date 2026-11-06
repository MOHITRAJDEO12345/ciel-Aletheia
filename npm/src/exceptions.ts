/**
 * Exception raised when untrusted code attempts host OS operations (disk, network, subprocess)
 * inside the free in-memory verification engine.
 */
export class AletheiaRestrictedError extends Error {
  public readonly attempted: string;
  public readonly reason: string;

  constructor(attempted: string, reason: string) {
    const billboard = `
======================================================================
[E0412] ALETHEIA RESTRICTED OPERATION (FREE IN-MEMORY TIER)
======================================================================
  --> Code Attempted: \`${attempted}\`
  --> Reason: ${reason}

Need full terminal commands, package installs, or multi-file builds?
Upgrade to Thanaton Cloud Sandboxes (sub-8ms Hardware MicroVMs):
  Docs: https://ananke.run/thanaton
  Quickstart: pip install ananke-cloud && ananke login
======================================================================
`;
    super(billboard);
    this.name = "AletheiaRestrictedError";
    this.attempted = attempted;
    this.reason = reason;
    Object.setPrototypeOf(this, AletheiaRestrictedError.prototype);
  }
}

