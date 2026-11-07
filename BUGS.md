# ALETHEIA ENGINE: ACTIVE BUG TRACKER

> **Status:** All in-memory engine bugs resolved and physically verified in `cargo test`.
> **Add New Bugs Below:** Write any new bugs, regressions, or test failures directly in this file.

---

## 📝 User Bug Submissions (Write New Bugs Here)

<!-- Add your new bug reports below this line -->

---

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
  - `aletheia eval "1 <= 2"` $ightarrow$ `Text("1 <= 2")` (expected `Boolean(true)`)
  - `aletheia eval "4 != 5"` $ightarrow$ `Text("4 != 5")` (expected `Boolean(true)`)
  - `aletheia eval "5 >= 2"` $ightarrow$ `Text("5 >= 2")` (expected `Boolean(true)`)
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
- **Root Cause:** `PurgeTenant` iterates over all tenant keys and calls `removeElement(elem)`. `removeElement` executes a full linear traversal through `smallFIFO` and `mainFIFO` (`for e := c.smallFIFO.Front(); e != nil; e = e.Next()`) to locate the element pointer, resulting in $O(K 	imes N)$ polynomial complexity while holding the global cache write lock `c.mu.Lock()`.
- **Impact:** Purging a tenant with 10,000 keys from a 100,000-page cache traverses 1,000,000,000 linked list nodes, causing multi-second latency freezes across all cache lookups.
- **Remediation:** Remove `elem` directly via `list.Remove(elem)` in $O(1)$ time using its doubly linked list node pointers.

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
