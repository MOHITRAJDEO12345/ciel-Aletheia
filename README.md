# Aletheia: Ultra-Low-Latency In-Memory Verification Engine
## Free Developer Wedge for AI Reasoning, RL Rollouts & Test-Time Compute

Aletheia is a standalone, fuel-metered in-memory bytecode and symbolic verification engine engineered for AI inference, LLM test-time compute, and reinforcement learning reward pipelines (PPO / GRPO / Best-of-N).

Licensed under **Apache 2.0**. Sub-microsecond execution latency.

---

### Verified Benchmark Performance

| Metric | Measured Score | Industry Standard (Wall-Clock Sandbox) | Advantage |
| :--- | :--- | :--- | :--- |
| **Evaluation Latency** | **1.09 µs / operation** | 20 ms – 150 ms | **>18,000x faster** |
| **Throughput** | **911,208 ops / sec** | 6 – 50 ops / sec | **>18,000x higher density** |
| **Symbolic Equivalence** | **< 10 µs** | 200 ms – 800 ms (SymPy/Z3) | **>20,000x faster** |
| **Infinite Loop Trap** | **< 5 µs** (fuel metering) | 2,000,000 µs (timeout wait) | **400,000x faster** |
| **Micro-Fuzzing (1,000 vectors)**| **262 µs** | 2,000 ms – 5,000 ms | **>7,500x faster** |
| **Numerical Precision Drift** | **0%** (exact BigInt & Rationals) | Undefined (IEEE-754 drift) | **Mathematically exact** |
| **Session Fork Latency** | **< 100 ns** (Micro-CoW) | 15 ms – 250 ms | **>150,000x faster** |
| **Memory Footprint** | **~64 KB** per worker | 15 MB – 50 MB per container | **>230x memory savings** |

---

### Core Architectural Primitives

1. **Instruction-Level Fuel Metering:** Replaces wall-clock timeout freezes with deterministic instruction counting. Traps runaway loops in **< 5 microseconds** without blocking the host thread.
2. **Symbolic Math Equivalence Canonicalizer:** Expands and normalizes algebraic polynomials and rational expressions in **< 10 µs**, verifying mathematical equivalence (e.g. $(x+1)^2 \equiv x^2 + 2x + 1$) without heavyweight CAS libraries.
3. **Deterministic RL Reward Telemetry:** Returns structured gas/fuel consumption, execution latency, and memory footprints for GRPO (Group Relative Policy Optimization) and PPO loss functions.
4. **Micro-CoW In-Memory Session Branching:** Enables sub-microsecond `.fork()` for MCTS (Monte Carlo Tree Search) and candidate rollout exploration without re-executing prefix steps.
5. **Exact Arbitrary-Precision Arithmetic:** Native BigInt combinatorics and BigRational fractions ($p/q$) eliminate precision loss in multi-step mathematical reasoning.
6. **Instant Boundary Fuzzer:** Generates and validates 1,000 numerical and logical boundary test vectors in **< 300 µs**.
7. **Zero-Copy Sidecar IPC:** Plugs directly into inference engines (vLLM / SGLang) via shared-memory ring buffers.

---

### The Unified Continuum: Free Wedge vs. Enterprise Cloud

| Capability | **Aletheia** (Free Open Source) | **Thanaton + Mnemon** (Enterprise Cloud) |
| :--- | :--- | :--- |
| **Workload Scope** | Math proofs, AST checks, RL rollouts, micro-fuzzing | Full coding agents, bash execution, web previews |
| **Execution Tier** | **In-Memory Bytecode (< 1.09 µs)** | **Hardware MicroVM (< 8 ms)** |
| **Memory Footprint**| **~64 KB** | **~15 MB** |
| **Filesystem & Bash**| ❌ None (Pure compute) | ✅ Full isolated rootfs with bash & sudo |
| **Package Installs**| ❌ None (Pure primitives) | ✅ Dynamic `pip install`, `npm install`, apt |
| **Database State** | ❌ None | ✅ Instant Zero-Copy Database Branching |
| **License / Pricing**| **100% Free & Open Source (Apache 2.0)** | **Metered vCPU-seconds (cloud.ananke.run)** |

---

### 1. Rust CLI Quickstart

```bash
# Build standalone release binary
cargo build --release

# Run performance benchmark (100,000 ops)
./target/release/aletheia benchmark

# Verify mathematical equivalence in < 10µs
./target/release/aletheia equiv "(x + 1)^2" "x^2 + 2*x + 1"

# Generate deterministic RL telemetry
./target/release/aletheia telemetry "x = 42"

# Evaluate exact combinatorics in < 50µs
./target/release/aletheia eval "comb(100, 50)"

# Run instant micro-fuzzer (1,000 boundary trials)
./target/release/aletheia fuzz 1000
```

---

### 2. Python SDK Quickstart

```bash
pip install aletheia-verifier
```

```python
from aletheia import Verifier, fuzz, equiv

v = Verifier(max_fuel=500_000)

# 1. Symbolic Math Equivalence (Verifies in < 15µs)
is_match = v.equiv("(x + 1)**2", "x**2 + 2*x + 1")
print("Equivalence verified:", is_match) # Output: True

# 2. Deterministic RL Telemetry for GRPO / PPO
telem = v.eval_with_telemetry("comb(20, 10)")
print("RL Telemetry:", telem)
# Output: <ExecutionTelemetry: result=184756, fuel=10, latency=67µs, memory=64B, verified=True>

# 3. Micro-CoW Session Forking for MCTS Tree Search (< 100ns)
branch_a = v.fork()
branch_b = v.fork()

# 4. Instant Micro-Fuzzer: 1,000 boundary trials in < 300µs
fuzz_res = fuzz(lambda x: (x + 1) != x, total_trials=1000)
print(fuzz_res)
# Output: <FuzzResult: 1000 cases verified in 262µs (Passed=True)>
```

---

### 3. Node.js & TypeScript SDK Quickstart

```bash
npm install @ananke/aletheia
```

```typescript
import { Verifier, fuzz, equiv } from "@ananke/aletheia";

const v = new Verifier();

// 1. Symbolic Math Equivalence
const isEq = v.equiv("(x + 1n)**2n", "x**2n + 2n*x + 1n");
console.log("Equivalent:", isEq); // Output: true

// 2. Deterministic RL Telemetry
const telem = v.evalWithTelemetry("comb(20, 10)");
console.log(`Telemetry: fuel=${telem.fuelConsumed}, latency=${telem.latencyUs}µs`);

// 3. Micro-CoW Session Branching (< 100ns)
const child = v.fork();

// 4. Micro-Fuzzing 1,000 cases in < 1ms
const fuzzRes = fuzz((x) => x + 1n !== x, 1000);
console.log(fuzzRes);
// Output: { totalCases: 1000, passed: true, durationUs: 557 }
```

---

### In-Terminal Billboard: Smart Restricted Exception
When untrusted agent code attempts host filesystem, network, or subprocess operations:
```python
v.eval("import subprocess; subprocess.run(['npm', 'test'])")
```
Output:
```text
======================================================================
[E0412] ALETHEIA RESTRICTED OPERATION (FREE IN-MEMORY TIER)
======================================================================
  --> Code Attempted: `import subprocess`
  --> Reason: Process spawning and shell commands require Thanaton MicroVMs

Need full terminal commands, package installs, or multi-file builds?
Upgrade to Thanaton Cloud Sandboxes (sub-8ms Hardware MicroVMs):
  Docs: https://ananke.run/thanaton
  Quickstart: pip install ananke-cloud && ananke login
======================================================================
```
