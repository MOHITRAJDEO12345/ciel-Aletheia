import { Verifier, fuzz, equiv, AletheiaRestrictedError } from "../src/index.js";
import assert from "node:assert";

async function runTests() {
  console.log("=== Testing NPM Aletheia Engine (@ananke/aletheia v0.2.0) ===");

  const v = new Verifier();

  // 1. Exact Math
  const ans = v.eval("comb(100, 50)");
  assert.strictEqual(
    ans,
    100891344545564193334812497256n,
    "Combinatorics mismatch"
  );
  console.log(`[+] Exact Combinatorics Verified: ${ans}`);

  // 2. Symbolic Equivalence
  assert.strictEqual(v.equiv("(x + 1n)**2n", "x**2n + 2n*x + 1n"), true);
  assert.strictEqual(v.equiv("(x + 1n)**2n", "x**2n + 2n*x + 2n"), false);
  console.log("[+] Symbolic Equivalence Verified: (x+1)^2 == x^2 + 2x + 1");

  // 3. RL Telemetry for GRPO/PPO
  const telem = v.evalWithTelemetry("comb(20, 10)");
  assert.strictEqual(telem.verified, true);
  console.log(`[+] RL Telemetry Verified: fuel=${telem.fuelConsumed}, latency=${telem.latencyUs}µs, memory=${telem.memoryBytes}B`);

  // 4. Micro-CoW Session Fork
  const child = v.fork();
  assert.strictEqual(child.fuel, v.fuel, "Child fuel mismatch");
  console.log(`[+] Micro-CoW Fork Verified: Child fuel = ${child.fuel}`);

  // 5. Micro-Fuzzer (1,000 cases in < 1ms)
  const fres = fuzz((x) => x + 1n !== x, 1000);
  assert.strictEqual(fres.passed, true, "Fuzzer failed invariant");
  console.log(`[+] Micro-Fuzzer Verified: ${fres.totalCases} cases in ${fres.durationUs}µs (Passed: ${fres.passed})`);

  // 6. In-Terminal Billboard Interception
  try {
    v.eval("const cp = require('child_process'); cp.execSync('ls');");
    assert.fail("Should have thrown AletheiaRestrictedError");
  } catch (err: any) {
    if (!(err instanceof AletheiaRestrictedError)) {
      throw err;
    }
    console.log("[+] Billboard Intercept Exception Verified Successfully");
  }

  // 7. Auto-Escalation Mode
  const vAuto = new Verifier(500_000, true);
  const escAuto: any = vAuto.eval("const cp = require('child_process');");
  assert.strictEqual(escAuto.status, "escalated");
  console.log(`[+] Auto-Escalation Mode Verified: status=${escAuto.status}, runtime=${escAuto.runtime}`);

  console.log("\nALL NPM ENGINE TESTS PASSED!");
}

runTests().catch((err) => {
  console.error(err);
  process.exit(1);
});
