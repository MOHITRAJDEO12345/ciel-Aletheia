# SOVEREIGN CLOUD & ALETHEIA PLATFORM: COMPREHENSIVE FORENSIC DEFECT REGISTER

> **Audit Status:** 59 critical, architectural, security, and algorithmic defects forensically verified across all layers.
> **Scope:** Aletheia In-Memory Core, Eschaton Gateway/Brain/Workers, Thanaton MicroVM & Control Plane, Mnemon-DB, Acheron, Lethe, Hyperion, and Kronos.
> **Zero Sugarcoating Mandate:** Every bug below identifies the exact failing component, file, line number, root cause, and concrete blast radius.

---

## 🚨 Table of Contents & Defect Summary
- **Section 1: Newly Added Feature Bugs in Recent Commits (`006f4c2`, `696ea67`, `f155f1d`)** (Bugs 1–13)
- **Section 2: Multi-Tenant SaaS Isolation, Security & Network Bypasses** (Bugs 14–26)
- **Section 3: Core Engine, Symbolic Math & Memory Safety Defects** (Bugs 27–42)
- **Section 4: Scale, Performance & Concurrency Degradations** (Bugs 43–59)

---

## Section 1: Newly Added Feature Bugs in Recent Commits (`006f4c2`, `696ea67`, `f155f1d`)

### [BUG-AUTH-04] Self-Provisioning Free Tier Escalation to Enterprise Without Payment or Auth
- **Component:** Eschaton Public Gateway Auth
- **Commit:** `f155f1d`
- **File:** `eschaton/gateway/main.go` (lines 438–442)
- **Root Cause:** In `handleSignup`:
  ```go
  tier := "free"
  if req.Tier == "startup" || req.Tier == "enterprise" {
      tier = req.Tier
  }
  ```
  The self-service signup endpoint blindly accepts `tier: "enterprise"` or `tier: "startup"` from the unauthenticated client JSON body. There is zero Stripe check, zero payment verification, and zero administrative authorization.
- **Impact:** Any unauthenticated caller on the public internet can register an Enterprise-tier tenant for free, unlocking elevated rate limits, quotas, and server resources.
- **Remediation:** Hardcode self-provisioned signups strictly to `"free"`; elevate to `"startup"` or `"enterprise"` only via authenticated Stripe webhook callbacks or administrative grant endpoints.

---

### [BUG-SEC-08] Emergency Tenant Quarantine Killswitch Bypass for Local / API Token Keys
- **Component:** Thanaton Control Plane Security Killswitch
- **Commit:** `006f4c2`
- **File:** `thanaton-control-plane/pkg/api/server.go` (lines 125–135)
- **Root Cause:** In `authenticateOrg`:
  ```go
  tok, err := s.repo.ValidateAPIToken(tokenStr)
  if err == nil {
      return s.repo.GetOrganization(tok.OrgID)
  }
  ```
  All tokens provisioned via `POST /api/v1/auth/signup` are stored and validated via `s.repo`. When an administrator activates an emergency tenant quarantine via `POST /api/v1/admin/tenant/quarantine`, `s.auth.QuarantineTenant()` flags the tenant in `s.auth`. However, `authenticateOrg` checks `s.repo` **first** and immediately returns the organization without ever querying `s.auth.IsTenantQuarantined()`.
- **Impact:** Quarantined malicious tenants can continue executing sandboxes, triggering builds, and creating projects without impediment using their existing API tokens.
- **Remediation:** Check `s.auth.IsTenantQuarantined(tok.OrgID)` immediately after successful token validation in `s.repo`.

---

### [BUG-SEC-09] Project Scoping Query Parameter Omission Bypass
- **Component:** Thanaton Control Plane Fine-Grained Key Scoping
- **Commit:** `006f4c2`
- **File:** `thanaton-control-plane/pkg/api/server.go` (lines 145–155)
- **Root Cause:** In `authenticateOrg`:
  ```go
  if cached.ProjectID != "" {
      projReq := r.URL.Query().Get("project_id")
      if projReq != "" && projReq != cached.ProjectID {
          return nil, fmt.Errorf("unauthorized: key is restricted to project %s", cached.ProjectID)
      }
  }
  ```
  The security policy only rejects requests if `projReq` is non-empty and mismatches `cached.ProjectID`. If the client omits `?project_id=` from the URL query, `projReq` is empty (`""`), and the check evaluates to false.
- **Impact:** An API key restricted to a specific project can access, list, or mutate resources across the entire organization simply by omitting the `project_id` query parameter.
- **Remediation:** If `cached.ProjectID != ""`, mandate that `projReq` must be present and must strictly match `cached.ProjectID`; reject all requests lacking the parameter or attempting organization-wide access.

---

### [BUG-CLI-02] Admin CLI Out-of-the-Box Connection Refused Due to Port Mismatch
- **Component:** Thanaton Admin CLI
- **Commit:** `006f4c2`
- **File:** `thanaton-control-plane/cmd/ananked/main.go` (lines 586, 1248)
- **Root Cause:** In `cmd/ananked/main.go:586`, `runDaemon` enforces `if adminPort == 9090 { adminPort = 9095 }` to avoid port collision with Mnemon-DB. However, in `handleAdminCLI` (line 1248):
  ```go
  serverURL := fs.String("server", "http://127.0.0.1:9090", "Admin API server URL")
  ```
  The default CLI target URL remains `http://127.0.0.1:9090`.
- **Impact:** Out-of-the-box administrative commands (`ananked admin cordon`, `drain`) fail immediately with `connection refused` unless the operator explicitly passes `--server=http://127.0.0.1:9095`.
- **Remediation:** Update the default flag in `handleAdminCLI` to `http://127.0.0.1:9095`.

---

### [BUG-UI-01] Hardcoded `is_quarantined: false` in User Quota API
- **Component:** Thanaton Control Plane User Telemetry
- **Commit:** `006f4c2`
- **File:** `thanaton-control-plane/pkg/api/server.go` (line 1980)
- **Root Cause:** In `handleUserQuotas`:
  ```go
  writeJSON(w, http.StatusOK, map[string]interface{}{
      "tenant_id":      org.ID,
      ...
      "is_quarantined": false,
      "checked_at":     time.Now().UTC().Format(time.RFC3339),
  })
  ```
  The endpoint returns a static constant `"is_quarantined": false`.
- **Impact:** Tenants under active security quarantine inspect their quotas and receive false confirmation that their status is normal and active.
- **Remediation:** Dynamically check `s.auth.IsTenantQuarantined(org.ID)` and return the true boolean state and reason.

---

### [BUG-AUTH-05] Nil Pointer Dereference Panic in `ValidateKey` on Key Seed Without Tenant
- **Component:** Thanaton Auth Service
- **Commit:** `006f4c2`
- **File:** `thanaton-control-plane/pkg/auth/auth.go` (line 313)
- **Root Cause:** In `ValidateKey`:
  ```go
  if quarantined, qRec := a.IsTenantQuarantined(cached.Tenant.ID); quarantined {
  ```
  `SeedKey` explicitly permits keys where `cached.Tenant == nil`. If `cached.Tenant` is nil, accessing `cached.Tenant.ID` causes a runtime panic.
- **Impact:** A seeded system token or tenant-less key crashes the daemon process upon the first validation call.
- **Remediation:** Add a guard `if cached.Tenant != nil` before accessing `cached.Tenant.ID`.

---

### [BUG-AUTH-06] Irreversible Key Loss Upon Unquarantine in Memory Repository
- **Component:** Thanaton Auth Service Quarantine Lifecycle
- **Commit:** `006f4c2`
- **File:** `thanaton-control-plane/pkg/auth/auth.go` (lines 170–175, 185–195)
- **Root Cause:** In `QuarantineTenant`, keys are permanently evicted from the cache via `delete(a.keyCache, keyHash)`. When `UnquarantineTenant` is executed, it deletes the tenant from `quarantinedTenants`, but never re-inserts the deleted keys into `keyCache`.
- **Impact:** In local, testing, or offline environments without an active Supabase backing store, unquarantining a tenant does not restore access; all previous API keys are permanently lost.
- **Remediation:** Move quarantined keys to a separate `quarantinedKeyCache` map during quarantine, and restore them to `keyCache` upon unquarantine.

---

### [BUG-VM-10] Relational Comparison Operators (`<=`, `>=`, `!=`) Silently Return Raw Strings
- **Component:** Aletheia In-Memory Core Engine
- **Commit:** `696ea67`
- **File:** `aletheia/src/engine/session.rs` (lines 275–310)
- **Root Cause:** `tokenize_expression` only handles single `=` (`Token::Op('=')` for `==`), `<`, and `>`. It does not tokenize `<=`, `>=`, or `!=`. When encountering `<=`, the `=` following `<` hits `return None`, causing `parse_shunting_yard` to fail and `parse_simple_expression` to silently return the raw expression string.
- **Physical Verification:**
  - `aletheia eval "1 <= 2"` -> `Text("1 <= 2")` (expected `Boolean(true)`)
  - `aletheia eval "4 != 5"` -> `Text("4 != 5")` (expected `Boolean(true)`)
  - `aletheia eval "5 >= 2"` -> `Text("5 >= 2")` (expected `Boolean(true)`)
- **Remediation:** Implement tokenization for multi-character comparison operators (`<=`, `>=`, `!=`) in `tokenize_expression` and emit corresponding VM opcodes.

---

### [BUG-SEC-10] GitHub Webhook HMAC Validation Completely Skipped if Secret Not Set
- **Component:** Thanaton Control Plane Webhooks
- **Commit:** `006f4c2`
- **File:** `thanaton-control-plane/pkg/api/server.go` (lines 1464–1478)
- **Root Cause:** In `handleGitHubWebhook`:
  ```go
  secret := os.Getenv("GITHUB_WEBHOOK_SECRET")
  if secret != "" {
      // Validate X-Hub-Signature-256
  }
  ```
  If `GITHUB_WEBHOOK_SECRET` is unset, the entire signature check block is bypassed.
- **Impact:** In environments where the environment variable is omitted, any anonymous internet caller can POST arbitrary payloads to `/api/v1/webhooks/github`, triggering build pipelines, deployments, and project creations with unvalidated tenant slugs.
- **Remediation:** Reject incoming webhooks with `401 Unauthorized` if no webhook secret is configured, or explicitly require a valid pre-shared secret header.

---

### [BUG-PERF-01] S3FIFO Cache Purge Catastrophic O(N^2) Latency Degradation
- **Component:** Thanaton CacheExt Engine
- **Commit:** `696ea67`
- **File:** `thanaton-control-plane/pkg/cacheext/s3fifo.go` (lines 310–345)
- **Root Cause:** `PurgeTenant` iterates over all tenant keys and calls `removeElement(elem)`. `removeElement` executes a full linear traversal through `smallFIFO` and `mainFIFO` (`for e := c.smallFIFO.Front(); e != nil; e = e.Next()`) to locate the element pointer, resulting in O(K * N) polynomial complexity while holding the global cache write lock `c.mu.Lock()`.
- **Impact:** Purging a tenant with 10,000 keys from a 100,000-page cache traverses 1,000,000,000 linked list nodes, causing multi-second latency freezes across all cache lookups.
- **Remediation:** Remove `elem` directly via `list.Remove(elem)` in O(1) time using its doubly linked list node pointers.

---

### [BUG-MEM-01] Retained Pointer Memory Leak in Skyloft Work-Stealing Fiber Queue
- **Component:** Thanaton Skyloft Userspace Scheduler
- **Commit:** `696ea67`
- **File:** `thanaton-control-plane/pkg/skyloft/fiber.go` (line 42)
- **Root Cause:** In `WorkerQueue.Steal()`:
  ```go
  f := q.fibers[0]
  q.fibers = q.fibers[1:]
  return f
  ```
  Reslicing a Go slice from index 1 retains the pointer at index 0 in the underlying array, preventing the garbage collector from reclaiming the stolen `Fiber` struct and its captured workload closures.
- **Impact:** High-throughput work-stealing continuously leaks memory references until the backing slice reaches reallocation capacity.
- **Remediation:** Explicitly nil-out the stolen slot before reslicing: `q.fibers[0] = nil; q.fibers = q.fibers[1:]`.

---

### [BUG-IO-01] Real-Time Telemetry Stream Buffering & 32MB Truncation in Gateway Proxy
- **Component:** Eschaton Public Gateway Proxy
- **Commit:** `696ea67`
- **File:** `eschaton/gateway/proxy.go` (lines 120–150)
- **Root Cause:** `ProxyPassthrough` forwards Server-Sent Events (SSE) telemetry using `io.Copy(w, io.LimitReader(resp.Body, 32*1024*1024))`. It never calls `w.(http.Flusher).Flush()`, causing Go HTTP transport buffers to accumulate data rather than streaming events in real time. Additionally, the stream is hard-capped at 32MB.
- **Impact:** Clients connecting to `/api/v1/telemetry/stream` experience event starvation (no events delivered until buffers fill) and premature connection closure after 32MB of log data.
- **Remediation:** Use an unbuffered chunked proxy reader with per-event `http.Flusher.Flush()` calls, and remove the artificial 32MB reader limit for persistent SSE connections.

---

### [BUG-MNM-03] Anonymous Database Claim Rejection for Unified Platform Keys
- **Component:** Mnemon-DB Auth & Tenant Management
- **Commit:** `696ea67`
- **File:** `mnemon-db/engine-go/pkg/auth/auth.go` (lines 305–315)
- **Root Cause:** In `ClaimAnonymousDatabase`:
  ```go
  targetRec, found := m.keys[targetAPIKey]
  if !found {
      targetRec, found = m.keys[HashKey(targetAPIKey)]
  }
  if !found {
      return nil, errors.New("target API key not found")
  }
  ```
  The claim routine only queries the local in-memory `m.keys` map. It does not fall back to `m.LookupSupabaseKey(targetAPIKey)`.
- **Impact:** Developers holding cross-product unified platform keys (`esk_live_...` or `thanaton_sec_...`) cannot claim anonymous databases; the API always rejects their keys with `"target API key not found"`.
- **Remediation:** Query `LookupSupabaseKey(targetAPIKey)` when `targetAPIKey` is not present in local `m.keys`.

---

## Section 2: Multi-Tenant SaaS Isolation, Security & Network Bypasses

### [BUG-SEC-01] Node.js/TypeScript SDK Allows Arbitrary Host Code Execution & Env Leaks
- **Component:** Aletheia TypeScript SDK
- **File:** `aletheia/npm/src/verifier.ts` (lines 95–104)
- **Root Cause:** In-memory fallback uses `new Function(...)` inside `eval()`.
- **Impact:** Running `v.eval("globalThis.process.env")` escapes the in-memory boundary and dumps host environment variables, process paths, and credentials.
- **Remediation:** Execute inside isolated Node.js `vm.createContext()` with null prototype and frozen primitives.

---

### [BUG-SEC-05] Unauthenticated Arbitrary Tenant Impersonation in Thanaton Edge Proxy
- **Component:** Thanaton Engine Reverse Proxy
- **File:** `thanaton-engine/pkg/proxy/proxy.go` (lines 215–225)
- **Root Cause:** `tenantID := r.Header.Get("X-Aether-Tenant")` is trusted blindly without authorization checks.
- **Impact:** Any unauthenticated internet caller can pass an arbitrary header (e.g. `X-Aether-Tenant: payment-processor`) to wake and proxy requests directly into private tenant microVMs.
- **Remediation:** Validate incoming requests against authenticated JWT tenant claims or pre-shared tenant API keys before routing or waking microVMs.

---

### [BUG-SEC-06] Unauthenticated Auto-Provisioning & DoS on Postgres Wire (Port 5432)
- **Component:** Mnemon-DB Postgres Wire Proxy
- **File:** `mnemon-db/engine-go/pkg/pgproxy/proxy.go` (lines 180–195)
- **Root Cause:** Incoming PostgreSQL connections specifying an unknown database/tenant trigger `p.runtime.Provision(..., MemoryMaxMB: 512, VCPUs: 2)`.
- **Impact:** Because port 5432 is publicly exposed on AWS EC2, an attacker can loop through random tenant names, exhausting host RAM, disk space, and cgroup limits.
- **Remediation:** Restrict automatic database provisioning to authenticated API endpoints; unknown tenants connecting over port 5432 must be rejected with `Code: 3D000 (database does not exist)`.

---

### [BUG-SEC-07] Universal Password-Free Superuser Access via `--auth=trust` on Public Postgres Wire
- **Component:** Mnemon-DB Database Engine & Transaction Pool
- **Files:** `mnemon-db/engine-go/pkg/sandbox/sandbox.go` (line 198) and `mnemon-db/engine-go/pkg/pgproxy/proxy.go` (lines 295–305)
- **Root Cause:** Data directories are initialized with `initdb ... --auth=trust`. In transaction pooling mode, the proxy replays cached `AuthenticationOk` packets directly to clients without checking passwords.
- **Impact:** Any user connecting from the public internet to `postgres.theeducode.com:5432` with username `postgres@<tenant>` gains immediate, unauthenticated superuser execution rights over that tenant's database.
- **Remediation:** Enforce `--auth=scram-sha-256`, require individual per-tenant master passwords, and challenge clients with `AuthenticationSASL` during proxy handshake.

---

### [BUG-SSRF-01] Server-Side Request Forgery via Unsanitized Execution Callback Webhooks
- **Component:** Eschaton Compilation Gateway
- **File:** `eschaton/brain/execute.go` (lines 70–95)
- **Root Cause:** `dispatchNativeWebhook` makes HTTP POST requests directly to user-supplied `callback_url` strings using a default `http.Client`. There are no IP blacklists restricting requests to private subnets, loopback, or the AWS EC2 metadata endpoint (`http://169.254.169.254/`).
- **Impact:** Callers can extract AWS instance IAM role credentials or trigger private control plane actions.
- **Remediation:** Implement an SSRF-safe HTTP transport that validates resolved IP addresses against loopback, link-local, and private ranges prior to dialing, and disallow redirects.

---

### [BUG-AUTH-03] Universal License Bypass via Unsigned Prefix Matching in All Sovereign Daemons
- **Component:** Sovereign Licensing Subsystem (Acheron, Lethe, Hyperion, Kronos)
- **Files:**
  - `acheron/pkg/authority/license.go` (lines 35–45)
  - `lethe/pkg/authority/license.go` (lines 34–44)
  - `hyperion/pkg/authority/license.go` (lines 34–44)
  - `kronos/pkg/authority/license.go` (lines 34–44)
- **Root Cause:** All four daemons evaluate Enterprise licenses using `strings.HasPrefix(a.licenseKey, "<PRODUCT>_SOVEREIGN_")` with zero cryptographic signature validation, zero HMAC check, zero public key verification, and zero expiration timestamp decoding.
- **Impact:** Passing `ACHERON_SOVEREIGN_dummy` or `LETHE_SOVEREIGN_fake` unlocks full "ENTERPRISE_SOVEREIGN" capabilities without paying for or obtaining a valid license.
- **Remediation:** Implement asymmetric cryptographic signature verification (Ed25519) binding signed claims to the authority verification routine.

---

### [BUG-TENANT-01] Hard Subnet Capacity Ceiling of 252 Tenants with Catastrophic IP Collisions
- **Component:** Thanaton Tenant Pool IP Allocator
- **File:** `thanaton-engine/pkg/pool/pool.go` (lines 465–478)
- **Root Cause:** `NextAvailableSubnetID()` uses a fixed loop:
  ```go
  for candidate := 2; candidate < 254; candidate++ {
      if !used[candidate] {
          return candidate
      }
  }
  return 254
  ```
  Once 252 tenants are provisioned, it always returns `254`.
- **Impact:** Every tenant from tenant #253 onward is allocated IP `172.16.254.2` on gateway `172.16.254.1`, causing severe network routing collisions, packet misdirection, and connection drops.
- **Remediation:** Expand IP address management (IPAM) to allocate across the full `172.16.0.0/12` range or use per-tenant network namespaces with overlapping RFC1918 space.

---

### [BUG-UID-01] Birthday Paradox POSIX UID Collision and Host Routing Table Contamination
- **Component:** Thanaton Dedicated NetNS Subsystem
- **File:** `thanaton-engine/pkg/netns/netns.go` (lines 39–55)
- **Root Cause:** `DeriveUID(vmid)` calculates `10000 + int(fnv32a(vmid) % 50000)`. In a 50,000-integer range, the Birthday Paradox causes a 50% probability of UID collision at just ~263 tenants.
- **Impact:** Colliding tenants share POSIX permissions in Jailer chroots and are assigned identical `/30` host interconnect IPs (`DeriveSubnetIPs`), causing Linux `RTNETLINK` conflicts.
- **Remediation:** Use an atomic stateful IPAM allocator stored in database/Redis rather than lossy 32-bit hash derivation.

---

### [BUG-ROAM-01] TCP Wire Session Destruction and Missing Restore in Acheron "Zero-Downtime" Roam
- **Component:** Acheron Immortal Socket Roaming Fabric
- **File:** `acheron/pkg/roam/repair_linux.go` (lines 20–50)
- **Root Cause:** `FreezeTCPConnection` assigns synthetic constants (`SeqNext = 1000000 + len`, `AckNext = 2000000`) and calls `tcpConn.Close()`, sending a TCP FIN packet that terminates the wire session. No `setsockopt(..., TCP_REPAIR)` is executed, and zero `RestoreTCPConnection` logic exists.
- **Impact:** Live connection migration physically destroys the client's TCP connection.
- **Remediation:** Implement real `TCP_REPAIR` socket options to freeze queues without generating FIN/RST packets, and implement `RestoreTCPConnection` to recreate kernel TCP state machine sequence numbers on target nodes.

---

### [BUG-DNS-01] Silent Packet Drop and Query Hang on Short UDP Packets in Acheron DNS
- **Component:** Acheron Sub-Microsecond Wire DNS Engine
- **Files:** `acheron/pkg/dns/engine.go` (lines 65–75) and `acheron/pkg/dns/wire.go` (lines 85–95)
- **Root Cause:** When `ParseQuestion` fails on a packet smaller than 12 bytes, `engine.go` invokes `BuildResponse(pkt, 12, nil, 0, RCodeFormErr)`. In `wire.go`, `BuildResponse` checks `if len(reqPacket) < qEndOffset { return nil }`. Since `len < 12`, it returns `nil`. `engine.go` does nothing, and client UDP queries hang until timeout.
- **Remediation:** Construct a minimal 12-byte DNS header with `RCodeFormErr` and write it to the client even if the query packet is truncated.

---

### [BUG-CDC-01] Lethe WAL Replication Slot Suicide via Absolute LSN Monotonic Miscalculation
- **Component:** Lethe CDC Engine & Circuit Breaker
- **Files:** `lethe/pkg/cdc/client.go` (lines 152–165) and `lethe/pkg/circuitbreaker/wal_breaker.go` (lines 100–120)
- **Root Cause:** In `cdc/client.go:153`:
  ```go
  lagBytes := int64(c.clientXLog)
  alert, err := c.breaker.DropSlotOnCatastrophicLag(c.cfg.SlotName, lagBytes, c.cfg.MaxKeepLagBytes, 0.0, ...)
  ```
  `clientXLog` is an absolute monotonic 64-bit byte offset in WAL generated since cluster creation—not replication lag.
- **Impact:** As soon as any production PostgreSQL cluster has generated 10GB of WAL cumulatively over its entire operational lifetime, Lethe drops its own replication slot via `pg_drop_replication_slot()`, permanently terminating replication.
- **Remediation:** Calculate true replication lag by querying `pg_current_wal_lsn() - confirmed_flush_lsn` from `pg_replication_slots`.

---

### [BUG-S3-01] S3 Multipart Stream Deadlock & Zero-Length Payload Rejection
- **Component:** Lethe S3 Pipe Subsystem
- **File:** `lethe/pkg/s3pipe/streamer.go` (lines 55–130)
- **Root Cause:** If any worker encounters an upload error and exits, the reader goroutine blocks forever trying to send to a full `tasks` channel, causing a permanent deadlock. Additionally, empty input streams send zero completed parts to `CompleteMultipartUpload`, which AWS S3 rejects with `400 Bad Request`.
- **Remediation:** Add context cancellation across workers and the reader upon error, and guard empty streams from multipart completion.

---

### [BUG-SCX-01] Hyperion Sched_ext Kernel Scheduler is a Complete No-Op Façade
- **Component:** Hyperion Linux Kernel Scheduling Engine
- **File:** `hyperion/pkg/scx/linux.go` (lines 35–45)
- **Root Cause:** `Dispatch(cpuID int, task *TaskDescriptor)` returns `nil` without executing. No eBPF programs, BPF maps, or `sched_ext_ops` are loaded into the Linux 6.12+ kernel.
- **Impact:** Tasks dispatched to Hyperion are completely ignored.
- **Remediation:** Integrate a real `libbpf` struct_ops loader binding `sched_ext_ops` into the kernel or remove deceptive marketing claims.

---

## Section 3: Core Engine, Symbolic Math & Memory Safety Defects

### [BUG-FFI-01] Unsigned Integer Underflow Memory Corruption in C API
- **Component:** Aletheia C FFI
- **File:** `aletheia/src/ffi/c_api.rs` (lines 60–65)
- **Root Cause:** `copy_len = bytes.len().min(out_len as usize - 1)`. Passing `out_len <= 0` underflows `usize` to `usize::MAX`, copying error strings into unallocated or null memory.
- **Remediation:** Enforce `if out_len <= 1 { return -1; }` guard.

---

### [BUG-AUTH-01] Unified Platform Key Breakdown (Cross-Product Auth Rejection)
- **Component:** Thanaton Control Plane & Mnemon-DB Auth
- **Files:** `thanaton-control-plane/pkg/auth/auth.go` (line 131), `mnemon-db/engine-go/pkg/auth/auth.go` (line 244)
- **Root Cause:** `README.md` documents `esk_live_...` as a unified sovereign key. The code explicitly blocked `esk_` prefixes with `"invalid key scope"`.
- **Remediation:** Enable unified key prefixes across all sovereign services.

---

### [BUG-SEC-03] Blacklist Incompletion Permits Trivial Sandbox Evasion in Billboard
- **Component:** Aletheia Billboard Security Boundary
- **File:** `aletheia/src/billboard/restricted.rs` (lines 125–140)
- **Root Cause:** Checked literal `import os`. An adversary using `from os import system`, `import urllib.request`, or `__import__('o'+'s')` completely evaded interception.
- **Remediation:** Tokenize imports via AST parser and inspect imported module namespaces.

---

### [BUG-NPM-01] Type Coercion Bug in NPM SDK: Polynomial Equivalence Returns False
- **Component:** Aletheia NPM SDK
- **File:** `aletheia/npm/src/verifier.ts` (lines 75–85)
- **Root Cause:** Schwartz-Zippel test points were BigInts (`{x: 2n}`). Calling `v.equiv("(x + 1)^2", "x^2 + 2*x + 1")` caused `2n + 1` to throw `TypeError: Cannot mix BigInt and other types`, falling back to string comparison and returning `false`.
- **Remediation:** Coerce literals to BigInt (`BigInt(1)`).

---

### [BUG-VM-01] Compound Binary Arithmetic Silently Falls Through to Raw Text
- **Component:** Aletheia In-Memory Core Engine
- **File:** `aletheia/src/engine/session.rs` (lines 190–205)
- **Root Cause:** Evaluator only supported a single operator (`1 + 2`). Compound expressions like `1 + 2 + 3` failed parsing and silently fell through to returning raw text strings.
- **Remediation:** Integrate full Shunting-Yard operator precedence parser.

---

### [BUG-VM-02] Polynomial Canonicalizer Crashes on Non-Polynomial Math
- **Component:** Aletheia Symbolic Math Engine
- **File:** `aletheia/src/engine/symbolic.rs` (lines 45–60)
- **Root Cause:** Monomial polynomial parser crashed on `sin(x)` or `cos(x)` with `Trailing tokens in expression`.
- **Remediation:** Return `Ok(false)` fallback when non-polynomial functions are encountered.

---

### [BUG-VM-03] Python SDK `eval()` Breaks on Standard Syntax & Assignments
- **Component:** Aletheia Python SDK
- **File:** `aletheia/python/aletheia/verifier.py` (lines 95–110)
- **Root Cause:** Native Python `eval()` passed empty `{"__builtins__": {}}`. Standard variable assignments, comprehensions with `range()`, and built-in functions threw runtime exceptions.
- **Remediation:** Expose safe builtins (`range`, `len`, `abs`, `min`, `max`) or route directly to Rust FFI core.

---

### [BUG-VM-04] Python Escalation Bridge is a Static Mock Stub
- **Component:** Aletheia Python SDK Escalation Bridge
- **File:** `aletheia/python/aletheia/bridge.py` (lines 5–20)
- **Root Cause:** Functions `upgrade_to_thanaton()` and `spawn_microvm()` returned hardcoded mock dictionaries without making network calls.
- **Remediation:** Implement real HTTP client calls to Thanaton (`POST /api/v1/sandboxes`).

---

### [BUG-VM-05] `FuelMeter::reset` Desyncs `consumed()` Metric Calculation
- **Component:** Aletheia Metering Engine
- **File:** `aletheia/src/engine/fuel.rs` (lines 45–55)
- **Root Cause:** Updated `remaining` atomic counter without resetting `initial_budget`, causing `consumed()` to underflow to zero.
- **Remediation:** Update both `remaining` and `initial_budget` atomically.

---

### [BUG-VM-06] Fuel Deduction Desync in Combinatorics (CPU Lockup Vector)
- **Component:** Aletheia Bytecode VM
- **File:** `aletheia/src/engine/bytecode.rs` (lines 160–170)
- **Root Cause:** `Op::Comb` deducted fuel proportional to `effective_k` (capped at 10,000). For `comb(1000000, 10000)`, it deducted 10,004 fuel units while locking the CPU for ~72ms generating a 5,000+ digit BigInt.
- **Remediation:** Deduct fuel proportional to BigInt bit operations ($O(k \cdot \log^2 n)$).

---

### [BUG-VM-07] Lexical Identifier Flaw Permits Numeric Variable Names
- **Component:** Aletheia Variable Binding
- **File:** `aletheia/src/engine/session.rs` (lines 140–150)
- **Root Cause:** `parse_assignment` allowed variables with purely numeric names (`123 = 456`), which could never be retrieved because numeric literals took precedence.
- **Remediation:** Enforce standard identifier grammar (`[a-zA-Z_][a-zA-Z0-9_]*`).

---

### [BUG-VM-08] Compound Assignment Operator Syntax Corruption
- **Component:** Aletheia Core Engine
- **File:** `aletheia/src/engine/session.rs` (lines 142–148)
- **Root Cause:** In `parse_assignment`, compound operators (`+=`, `-=`) resulted in variable names like `"x +"`.
- **Remediation:** Add operator symbols to the exclusion list in `parse_assignment`.

---

### [BUG-VM-09] Fictitious Memory Consumption Telemetry in RL Reward Model
- **Component:** Aletheia Reward Telemetry
- **File:** `aletheia/src/engine/session.rs` (lines 114–118)
- **Root Cause:** `memory_bytes` was computed as `std::mem::size_of_val(self) + (self.env.len() * 64)`. Heap allocations for large strings, BigInts, and rational numbers were completely ignored.
- **Remediation:** Traverse `self.env` and compute true heap sizes for stored objects.

---

### [BUG-FUZZ-01] Micro-Fuzzer Invariant is Hardcoded in CLI & MCP Server
- **Component:** Aletheia Fuzzing Engine
- **Files:** `aletheia/src/main.rs` (lines 75–85), `aletheia/src/ipc/mcp.rs` (lines 205–215)
- **Root Cause:** Invariant predicate was hardcoded to `|x| x.saturating_add(0) == x`. Users and MCP AI agents could not pass custom predicates.
- **Remediation:** Expose custom predicate input parameters via CLI and MCP tool schemas.

---

### [BUG-IPC-01] Shared Memory Ring Buffer is an Empty Skeleton
- **Component:** Aletheia IPC Subsystem
- **File:** `aletheia/src/ipc/shm_ring.rs`
- **Root Cause:** 33 lines of basic struct definitions with zero `mmap`, zero queue logic, and zero synchronization primitives.
- **Remediation:** Implement real POSIX shared memory (`shm_open`) or Windows named shared memory mapping.

---

### [BUG-KRN-01] Cryptographic Entropy Truncation & Non-Advancement in Kronos PRNG Trap
- **Component:** Kronos Deterministic Hypervisor Trap Handler
- **File:** `kronos/pkg/clock/traps.go` (lines 45–60)
- **Root Cause:** In `TrapGetrandom`:
  ```go
  copy(buf, h.entropyPool)
  return len(buf)
  ```
  `h.entropyPool` is a 32-byte SHA-256 digest. If the guest requests >32 bytes, bytes past offset 32 remain uninitialized zeros while the function falsely reports having generated `len(buf)` random bytes. Furthermore, `h.currentStep` is never incremented.
- **Remediation:** Implement a standard PRNG expansion loop (HKDF or ChaCha20) to fill `buf` completely, and advance `h.currentStep` on each read.

---

## Section 4: Scale, Performance & Concurrency Degradations

### [BUG-SPEC-01] Thanaton Speculation Engine is a Simulated Façade
- **Component:** Thanaton Speculation Subsystem
- **File:** `thanaton-control-plane/pkg/api/server.go` (lines 1963–1981)
- **Root Cause:** Generated synthetic timings (`12ms + i*4`) and mock test outputs (`140 passed in 0.01s`) instead of launching Firecracker MicroVMs.
- **Remediation:** Connect speculation endpoints to real background MicroVM worker pools.

---

### [BUG-LOG-01] Fake Log Injection When Redis Empty
- **Component:** Thanaton Telemetry
- **File:** `thanaton-control-plane/pkg/api/server.go` (lines 1050–1075)
- **Root Cause:** Injected synthetic log messages claiming Firecracker snapshots were restored in 76.4ms when no deployment existed.
- **Remediation:** Return empty array `[]` when no logs are found in Redis.

---

### [BUG-MET-01] Hardcoded Prometheus Metrics
- **Component:** Thanaton Prometheus Exporter
- **File:** `thanaton-control-plane/pkg/api/server.go` (lines 1380–1395)
- **Root Cause:** Emitted static string constants (`sabre_restore_throughput_gb_s 205.92`, `uslice_stall_reduction_pct 100.00`).
- **Remediation:** Wire Prometheus gauge metrics directly to atomic runtime telemetry counters.

---

### [BUG-CLI-01] CLI Silent Fallback on Missing Expression
- **Component:** Aletheia CLI
- **File:** `aletheia/src/main.rs` (lines 25–35)
- **Root Cause:** Evaluated `comb(100, 50)` by default if no expression was supplied to `aletheia eval`.
- **Remediation:** Print usage instructions and exit with non-zero code.

---

### [BUG-ESK-04] Extreme Latency Degradation Under Moderate Burst
- **Component:** Eschaton Compiler Queue
- **Endpoint:** `POST https://compiler.theeducode.com/v1/execute`
- **Root Cause:** Draining 15 concurrent requests took 8.29s (averaging ~828ms per request), debunking the 5,000 parallel submissions in 1.40s marketing claim.
- **Remediation:** Scale worker pool concurrency and avoid serializing requests on single worker channels.

---

### [BUG-ESK-01] Empty `test_cases` Array Causes 5.2-Second Timeout on `/v1/execute`
- **Component:** Eschaton Compiler Gateway
- **Endpoint:** `POST https://compiler.theeducode.com/v1/execute`
- **Root Cause:** Calling `/v1/execute` with `test_cases: []` hung for 5,242ms before returning null results.
- **Remediation:** Validate and default empty test case arrays immediately before queuing.

---

### [BUG-ESK-02] Large Source Code Payloads (200KB) Trigger Immediate 5.0s TLE
- **Component:** Eschaton File Injection
- **Endpoint:** `POST https://compiler.theeducode.com/v1/execute`
- **Root Cause:** Submitting 200KB of source code triggered a 5.0s TLE due to slow file injection into isolate boxes.
- **Remediation:** Use tmpfs memory boxes with direct streaming file injection.

---

### [BUG-ESK-03] Output Silently Discarded When `test_cases` Omitted
- **Component:** Eschaton Compiler Gateway
- **Endpoint:** `POST https://compiler.theeducode.com/v1/execute`
- **Root Cause:** Omitting `test_cases` returned `Results: null` and discarded captured stdout/stderr.
- **Remediation:** Automatically inject a default test case (`id: 1, stdin: ""`) when `test_cases` is omitted.

---

### [BUG-SEC-02] Billboard Substring Matching False Positives on Comments and Strings
- **Component:** Aletheia Billboard Security Boundary
- **File:** `aletheia/src/billboard/restricted.rs` (lines 140–155)
- **Root Cause:** Flagged harmless comments like `# open(file)` as restricted operations.
- **Remediation:** Strip comments and string literals prior to pattern scanning.

---

### [BUG-CFG-01] Missing `cdylib` in `Cargo.toml` Prevents Dynamic C/C++ FFI Linking
- **Component:** Aletheia Build Configuration
- **File:** `aletheia/Cargo.toml` (line 11)
- **Root Cause:** `crate-type = ["rlib"]` only built static Rust archives. `aletheia.dll` was never generated.
- **Remediation:** Set `crate-type = ["rlib", "cdylib"]`.

---

### [BUG-AUTH-02] Volatile In-Memory Token Persistence in Thanaton Control Plane
- **Component:** Thanaton Control Plane Repository
- **File:** `thanaton-control-plane/pkg/repo/repo.go` (lines 49–60)
- **Root Cause:** Defaulted to in-memory maps; tokens vanished upon daemon restart.
- **Remediation:** Mandate PostgreSQL persistent token storage in production.

---

### [BUG-MNM-01] Anonymous Tenant Database HTTP SQL Endpoint Lockdown
- **Component:** Mnemon-DB Management Daemon
- **File:** `mnemon-db/engine-go/cmd/mnemon-daemon/main.go` (line 375)
- **Root Cause:** `rec.AccountID != req.TenantID` check blocked anonymous databases from executing queries via the HTTP API.
- **Remediation:** Allow anonymous databases with valid claim tokens to execute SQL queries.

---

### [BUG-MNM-02] Lack of Self-Serve Provisioning Endpoint for Authenticated Mnemon Keys
- **Component:** Mnemon-DB Billing & Auth
- **File:** `mnemon-db/engine-go/pkg/billing/webhook.go` (lines 249–270)
- **Root Cause:** `/v1/billing/self-serve-provision` required an internal administrative secret.
- **Remediation:** Expose an unauthenticated self-provisioning endpoint for Free Tier developer keys.

---

### [BUG-USLICE-01] TOCTOU Race Condition in Hyperion Sub-Core CPU Pacer
- **Component:** Hyperion Micro-Slice CPU Allocation
- **File:** `hyperion/pkg/uslice/pacer.go` (lines 50–90)
- **Root Cause:** `remaining` was checked and granted without atomic CAS or mutex protection, allowing concurrent threads to consume multiples of their millicore limit.
- **Remediation:** Use an atomic CAS loop on `ConsumedInWinNs` to reserve time slices atomically.

---

### [BUG-LANG-01] Unsupported Standard Judge0 Language IDs (Go=60, Rust=73)
- **Component:** Eschaton Language Dispatcher
- **File:** `eschaton/brain/execute.go` (lines 380–390)
- **Root Cause:** The switch statement omitted standard Judge0 IDs for Go (`60`) and Rust (`73`), returning `HTTP 400: Unsupported Language ID` (physically verified on `compiler.theeducode.com`).
- **Remediation:** Map standard Judge0 IDs 60 (Go) and 73 (Rust) to worker dispatch channels.

---

### [BUG-CONC-01] Thundering Herd Reverse Proxy Connection Purge in Thanaton Edge Proxy
- **Component:** Thanaton Edge Reverse Proxy
- **File:** `thanaton-engine/pkg/proxy/proxy.go` (lines 280–305)
- **Root Cause:** When multiple concurrent requests arrived for a sleeping microVM, all threads woke the VM and sequentially executed `p.PurgeTenant(tenantID)`, repeatedly closing active TCP sockets out from under in-flight requests.
- **Remediation:** Only purge connections on running-to-sleeping state transitions and use single-flight awakening.

---
