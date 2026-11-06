import { Verifier, fuzz, AletheiaRestrictedError } from "../src/index.js";
import assert from "node:assert";

async function runTests() {
  console.log("=== Testing NPM Aletheia Engine (@ananke/aletheia) ===");

  const v = new Verifier();

  // 1. Exact Math
  const ans = v.eval("comb(100, 50)");
  assert.strictEqual(
    ans,
    100891344545564193334812497256n,
    "Combinatorics mismatch"
  );
  console.log(`[+] Exact Combinatorics Verified: ${ans}`);

  // 2. Micro-CoW Session Fork
  const child = v.fork();
  assert.strictEqual(child.fuel, v.fuel, "Child fuel mismatch");
  console.log(`[+] Micro-CoW Fork Verified: Child fuel = ${child.fuel}`);

  // 3. Micro-Fuzzer (1,000 cases in < 1ms)
  const fres = fuzz((x) => x + 1n !== x, 1000);
  assert.strictEqual(fres.passed, true, "Fuzzer failed invariant");
  console.log(`[+] Micro-Fuzzer Verified: ${fres.totalCases} cases in ${fres.durationUs}µs (Passed: ${fres.passed})`);

  // 4. In-Terminal Billboard Interception
  try {
    v.eval("const cp = require('child_process'); cp.execSync('ls');");
    throw new Error("Should have thrown AletheiaRestrictedError");
  } catch (err: any) {
    if (!(err instanceof AletheiaRestrictedError)) {
      throw err;
    }
    console.log("[+] Billboard Intercept Exception Verified Successfully:");
    console.log(err.message);
  }

  // 5. Escalation bridge
  const esc = v.escalate("test-key");
  assert.strictEqual(esc.status, "escalated");
  assert.strictEqual(esc.runtime, "thanaton-microvm");
  console.log(`[+] Escalation Bridge Verified: status=${esc.status}, latency=${esc.bootLatencyMs}ms`);

  console.log("\nALL NPM ENGINE TESTS PASSED!");
}

runTests().catch((err) => {
  console.error(err);
  process.exit(1);
});
