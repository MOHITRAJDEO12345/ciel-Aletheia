# SOVEREIGN CLOUD & ALETHEIA PLATFORM: ACTIVE BUG TRACKER

> **Status:** 16 critical & major bugs resolved and forensically verified in production (November 7, 2026).
> **Add New Bugs Below:** Write any new bugs, regressions, or test failures directly in this file.

---

## Active & Pending Bugs

### [BUG-SPEC-01] Thanaton Speculation Engine Phase 2 Uses Heuristic Pruning
- **Component:** Thanaton Control Plane Speculation Engine
- **File:** `thanaton-control-plane/pkg/api/server.go` (lines 1913–2055)
- **Status:** Phase 1 in-memory AST syntax pruning is live; Phase 2 microVM runner dispatch pool is currently being wired to asynchronous background tasks.

### [BUG-IPC-01] Shared Memory Ring Buffer Direct Hardware Mapping
- **Component:** Aletheia IPC Subsystem
- **File:** `aletheia/src/ipc/shm_ring.rs`
- **Status:** Pending bare-metal Intel IAA / GPU direct DMA hardware testbed.

### [BUG-ESK-02] Large Source Code Payloads (200KB) Trigger TLE During File Injection
- **Component:** Eschaton Compiler Sandbox
- **Endpoint:** `POST https://compiler.theeducode.com/v1/execute`
- **Status:** Investigating filesystem serialization bottleneck for payloads > 150KB.

### [BUG-ESK-04] Extreme Latency Degradation Under High-Concurrency Burst
- **Component:** Eschaton Compiler Queue
- **Endpoint:** `POST https://compiler.theeducode.com/v1/execute`
- **Status:** Worker thread pool concurrency scaling scheduled for cluster optimization.

---

## 📝 User Bug Submissions (Write New Bugs Here)

<!-- Add your new bug reports below this line -->
