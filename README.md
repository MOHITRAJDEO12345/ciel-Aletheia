# Aletheia: Ultra-Low-Latency In-Memory Verification Engine
## The Free Open-Source Developer Wedge for AI Reasoning & RL Rollouts

Aletheia is a standalone, fuel-metered in-memory bytecode and symbolic verification engine engineered for AI inference, LLM test-time compute, and reinforcement learning reward pipelines (PPO / GRPO / Best-of-N).

Licensed under **Apache 2.0**. Zero external dependencies. Sub-microsecond execution latency.

---

### Why Aletheia Exists: Solving the 5 LLM Execution Bottlenecks
1. **The Halting Problem / Wall-Clock Freezes:** Replaces 2-second wall-clock timeouts with **Instruction Fuel Metering**. Traps infinite loops in **< 5 microseconds**.
2. **Stateless Amnesia:** Introduces **Micro-CoW In-Memory Session Branching** for tree search (MCTS / Best-of-N) without re-running earlier steps.
3. **Floating-Point Betrayal:** Native **Arbitrary-Precision Rationals ($p/q$) and BigInts** prevent IEEE-754 precision drift.
4. **Fuzzing Hypotheses:** Instant **Micro-Fuzzing** evaluates 1,000 boundary edge cases in **< 2 milliseconds**.
5. **Zero-Copy Sidecar IPC:** Plugs directly into vLLM / SGLang GPU workers via `/dev/shm` ring buffers (< 150µs round-trip).

---

### The Two-Tier Continuum: Free Wedge vs. Commercial Cloud

| Capability | **Aletheia** (Free & Open Source) | **Thanaton + Mnemon** (Enterprise Cloud) |
| :--- | :--- | :--- |
| **Primary Workload** | Math proofs, AST checks, RL reward rollouts | Full coding agents, bash execution, web previews |
| **Startup Overhead** | **< 100 nanoseconds** (In-Memory) | **< 8 milliseconds** (Hardware Firecracker VM) |
| **Memory Footprint** | **~64 KB** per execution | **~15 MB** per sandbox |
| **Filesystem & Bash** | ❌ None (Pure compute) | ✅ Full isolated rootfs with bash & sudo |
| **Package Installs** | ❌ None (Pure primitives) | ✅ Dynamic `pip install`, `npm install`, apt |
| **Database State** | ❌ None | ✅ Instant Copy-on-Write Postgres branching |
| **Cost** | **100% Free & Open Source (Apache 2.0)** | **Metered vCPU-seconds (cloud.ananke.run)** |

---

### Quickstart (Rust CLI)
```bash
# Build standalone binary
cargo build --release

# Run instant calculation
./target/release/aletheia eval "comb(100, 50)"

# Run instant micro-fuzzer (1,000 cases in < 2ms)
./target/release/aletheia fuzz 1000

# Run performance benchmark
./target/release/aletheia benchmark
```

---

### Quickstart (Python)
```bash
pip install aletheia-verifier
```

```python
from aletheia import Verifier

v = Verifier(max_fuel=100_000)
# Evaluates in < 50µs with exact rational arithmetic
result = v.eval("math.comb(100, 50)")
print("Result:", result)

# Branch in sub-microseconds for MCTS
branch_a = v.fork()
branch_b = v.fork()

# Instant micro-fuzzer: verifies 1,000 edge cases in < 2ms
fuzz_res = v.fuzz(lambda x: x + 0 == x, total_trials=1000)
print(fuzz_res)
```

---

### The In-Terminal Billboard: Smart Restricted Exception
When an agent attempts disk writes, subprocess spawning, or network access:
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
Upgrade to Thanaton Cloud Sandboxes (sub-10ms hardware Firecracker microVMs):
  Docs: https://ananke.run/thanaton
  Quickstart: pip install ananke-cloud && ananke login
======================================================================
```
