# Discovery Classification Guide

When a discovery is identified during implementation (via the discovery-check hook), classify it using the decision tree below. Each discovery gets two labels: a **classification** (what kind of work it represents) and an **urgency** (how soon it's needed).

## Decision Tree

```
START: You've identified something that wasn't planned.

Q1: Does this affect multiple tracks or the whole system?
│
├─ YES ─┐
│       Q2: Is it a behavioral rule/constraint (HOW things should behave)?
│       ├─ YES → CROSS_CUTTING_CHANGE
│       │         (e.g., "all services must use structured logging")
│       │
│       └─ NO → Is it a structural change (WHAT components exist or how they connect)?
│           ├─ YES → ARCHITECTURE_CHANGE
│           │         (e.g., "we need a message broker, not direct HTTP calls")
│           │
│           └─ NO → Re-evaluate. It probably IS behavioral or structural.
│
└─ NO (affects one track or a small area) ─┐
        │
        Q3: Does it belong in an existing track's scope?
        │
        ├─ YES ─┐
        │       Q4: How much work is it?
        │       ├─ < 5 tasks, same tech stack → TRACK_EXTENSION
        │       │   (append a patch phase to the existing track's plan)
        │       │
        │       └─ 5+ tasks, or requires different tech → NEW_TRACK
        │           (it's big enough to be its own track)
        │
        └─ NO ─┐
                Q5: Is it a dependency or contract issue?
                ├─ "I need output from another track that isn't available yet"
                │   → NEW_DEPENDENCY
                │
                ├─ "Another track's API/event doesn't match what was specified"
                │   → INTERFACE_MISMATCH
                │
                └─ "This is entirely new functionality not in any track"
                    → NEW_TRACK
```

---

## Classifications

### 1. NEW_TRACK

A new track needs to be created. The work is substantial enough (5+ tasks) or distinct enough (different tech, different team concern) that it doesn't belong in an existing track.

**Action during sync:** Generate brief.md and metadata.json. Insert into DAG. Re-sequence waves. Conductor generates spec.md and plan.md when the developer picks up the track.

**Examples:**

- **Clear case:** During implementation of the API track, you discover the system needs WebSocket support for real-time notifications. This requires a WebSocket server, Redis pub/sub, client SDK, and reconnection logic — clearly its own track.

- **Clear case:** The product mentions "admin dashboard" as a minor note, but implementation reveals it needs its own auth flow, CRUD for every entity, audit logging UI, and user management. Too large for a TRACK_EXTENSION.

- **Ambiguous → NEW_TRACK (not TRACK_EXTENSION):** You're building the auth track and realize OAuth2 PKCE flow for the mobile app is fundamentally different from the web session flow. It shares some infrastructure (token validation) but needs its own middleware, redirect handling, and token refresh logic. Even though it's "auth," it's 8+ tasks with different concerns — make it a new track.

### 2. TRACK_EXTENSION

A small addition (< 5 tasks) to an existing track, using the same technology and fitting naturally into its scope. Becomes a patch phase in the track's plan.md.

**Action during sync:** Append a new phase to the existing track's plan.md. If track is COMPLETE, set state to NEEDS_PATCH.

**Examples:**

- **Clear case:** The API track's spec says "CRUD for resources," but during implementation you realize you also need a bulk-delete endpoint. It's 2 tasks (endpoint + tests) using the same framework, clearly part of this track.

- **Clear case:** The auth track is complete, and a new cross-cutting constraint requires adding rate limiting to the login endpoint. It's 3 tasks (middleware, config, tests) — a patch phase, not a new track.

- **Ambiguous → TRACK_EXTENSION (not NEW_TRACK):** The frontend track needs a toast notification component that wasn't in the spec. It's 3 tasks (component, integration with error handling, tests). Even though "notifications" could be its own domain, this is a UI component within the existing frontend scope.

### 3. NEW_DEPENDENCY

A dependency between tracks that wasn't captured in the original DAG. Track A needs something from Track B that wasn't specified.

**Action during sync:** Add edge to dependency-graph.md. Run `validate_dag.py` to check for cycles. If cycle detected, reclassify as ARCHITECTURE_CHANGE. Flag for developer review.

**Examples:**

- **Clear case:** The email notification track assumed it could call the user service API to get email addresses, but that API isn't in the user service's interfaces.md. The email track depends on the user track providing a `/users/{id}/email` endpoint.

- **Clear case:** The reporting track needs access to the event store, but the event store is built by the infrastructure track which is in a later wave. The reporting track is blocked.

- **Ambiguous → NEW_DEPENDENCY (not INTERFACE_MISMATCH):** The frontend track expects a `/v1/dashboard/summary` aggregation endpoint that no backend track owns. This isn't a mismatch in an existing interface — it's a missing dependency. The frontend needs a new endpoint created, and which track should own it needs to be decided.

### 4. CROSS_CUTTING_CHANGE

A behavioral rule or constraint that applies across multiple tracks. Gets version-appended to `architect/cross-cutting.md`.

**Action during sync:** Version-append to cross-cutting.md. NOT_STARTED tracks get regenerated headers. IN_PROGRESS tracks pick up via constraint-update-check hook. COMPLETE tracks get NEEDS_PATCH state with a patch phase.

**Examples:**

- **Clear case:** During the API track, you realize all services should return cache-control headers on read endpoints. This applies to every service that exposes an HTTP API — it's a behavioral constraint, not specific to one track.

- **Clear case:** A security review during the auth track reveals that all services must log authentication failures with a specific structured format for the SIEM. This is a logging convention that applies everywhere.

- **Ambiguous → CROSS_CUTTING_CHANGE (not ARCHITECTURE_CHANGE):** You discover that all event consumers should use a dead-letter queue for failed messages. This is a behavioral rule ("use DLQ") not a structural change (it doesn't add new components or change how services connect). Even though it requires infrastructure (the DLQ), the discovery is about the behavioral constraint.

### 5. ARCHITECTURE_CHANGE

A structural change to the system architecture — new components, changed communication patterns, or fundamental design revisions. Requires developer review before any action.

**Action during sync:** Present to developer with analysis. Do NOT auto-apply. Developer decides whether to accept, modify, or defer. If accepted, may trigger re-decompose of affected areas.

**Examples:**

- **Clear case:** The original architecture uses synchronous HTTP between services, but under load testing, you discover cascading timeout failures. The recommendation is to switch to asynchronous messaging via a message broker for non-critical paths. This changes how services communicate — a structural change.

- **Clear case:** The system was designed as 3 services, but implementation reveals the "core" service handles 5 unrelated domains and should be split into 3 separate services. This changes the component map.

- **Ambiguous → ARCHITECTURE_CHANGE (not CROSS_CUTTING_CHANGE):** You discover the system needs a shared cache layer (Redis) that wasn't in the original architecture. While "use caching" could be a behavioral constraint, the discovery is that a new infrastructure component needs to exist and multiple services need to connect to it. The structural addition of Redis is the core change.

### 6. INTERFACE_MISMATCH

A contract specified in `architect/interfaces.md` doesn't match what was actually implemented. The producer's implementation differs from what consumers expect.

**Action during sync:** Present to developer with specific differences. If the spec is wrong, update interfaces.md. If the implementation is wrong, the producing track needs to fix it. BLOCKING urgency if a consumer is currently implementing against the contract.

**Examples:**

- **Clear case:** interfaces.md says the user service returns `{ "user": { "id": "uuid", "email": "string" } }` but the actual implementation returns `{ "data": { "userId": "uuid", "emailAddress": "string" } }`. Field names and envelope structure don't match.

- **Clear case:** The event contract says `order.completed` events include a `total_amount` field, but the order service publishes `order.completed` without it — the field was moved to a separate `order.invoiced` event.

- **Ambiguous → INTERFACE_MISMATCH (not NEW_DEPENDENCY):** The auth track's `/v1/auth/me` endpoint exists and works, but returns user roles as a flat list `["admin", "editor"]` while the API track expected a structured format `[{"role": "admin", "scope": "org:123"}]`. The endpoint exists (not a missing dependency) — the problem is the contract doesn't match reality.

---

## Urgency Levels

After classifying the type, assess urgency:

### BLOCKING

Work cannot continue on the current or a dependent track without resolving this.

**Criteria — any of:**
- The current track is stuck and cannot proceed to the next task
- A dependent track in the same wave is blocked
- An INTERFACE_MISMATCH on an endpoint being actively consumed
- A cycle would be introduced into the dependency graph

**Action:** Notify developer immediately. Pause affected work until resolved.

**Examples:**
- Track 07 needs the auth middleware from Track 03, but Track 03 isn't complete and the mock doesn't cover the needed behavior. Track 07 cannot implement its protected endpoints.
- The dependency graph would have a cycle if the new edge is added. The DAG must be restructured before proceeding.

### NEXT_WAVE

Not blocking current work, but must be resolved before the next wave can start.

**Criteria — any of:**
- A track in the next wave depends on this discovery being resolved
- A cross-cutting change that affects next-wave tracks
- A new track that needs to be sequenced into the next wave

**Action:** Log discovery. Process during wave sync. Include in wave completion quality gate.

**Examples:**
- A new cross-cutting constraint (caching) was added. Current-wave tracks can finish without it, but next-wave tracks should include it from the start.
- A new track for WebSocket support was discovered. It doesn't block current API work, but the real-time features in Wave 4 depend on it. It should be added to Wave 3.

### BACKLOG

Nice to have. Not blocking anything. Can be addressed after current priorities.

**Criteria — all of:**
- No track is currently blocked or will be blocked
- The improvement is valuable but not urgent
- It can be deferred without affecting system correctness

**Action:** Log discovery. Process during next sync. Low priority in wave planning.

**Examples:**
- During the API track, you notice that adding a caching layer for the `/v1/resources` list endpoint would improve performance, but the current response times are acceptable (< 200ms).
- You realize a CLI admin tool would be useful for operations, but all current functionality works through the web UI.
- The frontend could benefit from a component library track for design consistency, but individual tracks are already building components that work.

---

## Distinguishing Ambiguous Cases: Quick Checks

| Ambiguity | Resolution |
|---|---|
| TRACK_EXTENSION vs NEW_TRACK | Count the tasks. < 5 tasks same tech = extension. 5+ or different tech = new track. |
| CROSS_CUTTING_CHANGE vs ARCHITECTURE_CHANGE | Does it change HOW things behave (constraint)? Or WHAT exists / how things connect (structure)? |
| NEW_DEPENDENCY vs INTERFACE_MISMATCH | Does the endpoint/event exist? Yes but wrong = mismatch. No = dependency. |
| BLOCKING vs NEXT_WAVE | Can you continue your current task right now? No = blocking. Yes but downstream needs it = next_wave. |
| NEW_TRACK vs NEW_DEPENDENCY | Is the work "build something new" or "I need something from an existing track"? Build = new track. Need = dependency. |
