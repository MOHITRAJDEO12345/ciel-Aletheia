# Aletheia Python SDK (`aletheia-verifier`)
## Ultra-Low-Latency In-Memory Verification Engine for AI Reasoning & RL Rollouts

Aletheia is a fuel-metered in-memory bytecode and symbolic verification engine engineered for AI inference, LLM test-time compute, and reinforcement learning reward pipelines (PPO / GRPO / Best-of-N).

Licensed under **Apache 2.0**.

---

### Key Benchmarks & Performance Metrics

* **In-Memory Evaluation Latency:** **~1.09 µs per operation** (100,000 evaluations in 109.74ms)
* **Evaluation Throughput:** **911,208 ops/sec**
* **Runaway Loop Trapping:** **< 5 µs** via instruction fuel metering
* **Micro-Fuzzing:** **1,000 boundary vectors in < 262 µs**
* **Precision Drift:** **0%** (exact BigInt and arbitrary-precision rational arithmetic)
* **Session Fork Overhead:** **Sub-microsecond Copy-on-Write branching** for tree search

---

### Installation
```bash
pip install aletheia-verifier
```

---

### Quickstart

```python
from aletheia import Verifier, fuzz

# 1. Initialize verifier with fuel metering
v = Verifier(max_fuel=500_000)

# 2. Exact Combinatorics & Symbolic Math (zero IEEE-754 float drift)
res = v.eval("comb(100, 50)")
print("Exact result:", res)
# Output: 100891344545564193334812497256

# 3. Micro-CoW Session Branching for MCTS Tree Search
branch_a = v.fork()
branch_b = v.fork()

# 4. Instant Micro-Fuzzer: 1,000 boundary cases in < 300µs
fuzz_result = fuzz(lambda x: (x + 1) != x, total_trials=1000)
print(fuzz_result)
# Output: <FuzzResult: 1000 cases verified in 262µs (Passed=True)>
```

---

### In-Terminal Billboard: Smart Restricted Exception
When untrusted code attempts host OS operations (disk, subprocess, network):
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

