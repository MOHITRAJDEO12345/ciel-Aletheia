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
  Signup: POST https://compiler.theeducode.com/v1/auth/signup
  Control Plane: https://control.theeducode.com
  Quickstart: import thanaton; client = thanaton.Client(api_key="esk_live_...")
======================================================================
```

---

### The Escalation Continuum & Live Developer Quickstart

When an autonomous agent transitions from in-memory math, logic, and AST checks to full hardware compilation, shell commands, or database transactions, it escalates to the live **Ananke Sovereign Cloud**.

#### Step 1: Self-Provisioning Free Tier API Key (Zero Credit Card Required)
Any autonomous agent or developer can programmatically register via the public API:

```http
POST https://compiler.theeducode.com/v1/auth/signup
Content-Type: application/json

{
  "email": "agent@theeducode.com",
  "name": "Autonomous Agent"
}
```
*(Fallback Direct IP: `http://13.126.155.62:8080/v1/auth/signup`)*

**Response (`201 Created`):**
```json
{
  "status": "created",
  "api_key": "esk_live_5eb8320ccda8e5e7d58ae3497dfea40ed9ec18db99948486",
  "tenant_id": "cf04170a-7a86-4064-af5f-51a9d6c60fdf",
  "tier": "free",
  "credits": 200,
  "daily_quota": 100,
  "rate_limit_rps": 10,
  "message": "Account provisioned successfully on Free Starter Tier (200 Credits, 100 calls/day)"
}
```

The issued key is a **unified platform key** that grants access across all three live engines.

---

#### Step 2: Multi-Language Code Compilation & Execution (Eschaton)
Execute code across 7 native languages in isolated Linux kernel sandboxes with pre-compiled headers:

```python
import urllib.request, json

payload = {
    "source_code": "print('Hello from Eschaton!')",
    "language_id": 71,  # 71 = Python, 54 = C++20, 62 = Java 21, 63 = Node.js, 60 = Go, 73 = Rust
    "test_cases": [
        {"id": 1, "stdin": "", "expected_output": "Hello from Eschaton!"}
    ]
}

req = urllib.request.Request(
    "https://compiler.theeducode.com/v1/execute", # or http://13.126.155.62:8080/v1/execute
    data=json.dumps(payload).encode(),
    headers={
        "Content-Type": "application/json",
        "X-API-Key": "<your_provisioned_api_key>"
    },
    method="POST"
)

with urllib.request.urlopen(req) as resp:
    result = json.loads(resp.read().decode())
    print("Execution Result:", result)
```

**Check Account Balance & Quota:**
```bash
curl -H "X-API-Key: <your_api_key>" https://compiler.theeducode.com/v1/account
```

---

#### Step 3: Hardware MicroVM Sandboxes (Thanaton)
Provision full KVM hardware microVM sandboxes with dedicated vCPU, memory quotas, and scale-to-zero snapshots:

```python
import thanaton

client = thanaton.Client(
    base_url="https://control.theeducode.com",  # or http://13.126.155.62:4000
    api_key="<your_provisioned_api_key>"
)

# 1. Provision a 256MB MicroVM (Free tier allows up to 3 projects)
sbx = client.sandboxes.create(runtime="python", memory_mb=256)
print(f"Booted Sandbox {sbx.id} (Status: {sbx.state})")

# 2. Check tenant allocations & whoami
whoami = client._request("GET", "/api/v1/auth/whoami")
print("Tenant Status:", whoami)
```

**Parallel MCTS Multiverse Speculation Engine (`POST /api/v1/speculate`):**
Evaluate candidate patches in parallel with sub-50µs in-memory AST syntax pruning before touching hardware microVMs:
```bash
curl -X POST https://control.theeducode.com/api/v1/speculate \
  -H "Content-Type: application/json" \
  -d '{
    "candidates": [
      {"id": "branch-1", "patch": "def solve():\n    return 42", "strategy": "ATOMIC_CAS"},
      {"id": "branch-2", "patch": "while True: pass", "strategy": "UNBOUNDED"}
    ],
    "test_command": "pytest"
  }'
```

---

#### Step 4: Serverless PostgreSQL & Zero-Copy Branching (Mnemon-DB)
Connect directly via standard PostgreSQL wire protocol or management HTTP endpoints:
- **Postgres Wire Protocol:** `postgres.theeducode.com:5432` (or `3.6.127.83:5432`)
- **HTTP Studio & Branching API:** `https://db.theeducode.com` (or `http://3.6.127.83:9090`)

---

### Free Tier Limits & Status Code Guide

| Boundary | Free Tier Limit | HTTP Code When Exceeded | Response Payload |
| :--- | :--- | :--- | :--- |
| **Credit Balance** | 200 Starter Credits | **`402 Payment Required`** | `{"error": "payment_required", "message": "tenant credit balance exhausted"}` |
| **Daily Calls** | 100 Calls / Day | **`429 Too Many Requests`** | `{"error": "quota_exceeded", "Retry-After": "3600"}` |
| **MicroVM Sandboxes**| Max 3 Projects | **`402 Payment Required`** | `{"code": "PROJECT_QUOTA_EXCEEDED", "max_allowed": 3}` |
| **Rate Limit** | 10 RPS | **`429 Too Many Requests`** | `{"error": "rate limit exceeded for your tier"}` |

