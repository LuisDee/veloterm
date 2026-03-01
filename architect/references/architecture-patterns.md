# Architecture Patterns: Signal-to-Pattern Mapping

This is Architect's built-in knowledge base for pattern recognition. During architecture research (Step 3 of decompose), extract signals from project inputs and match them to patterns below.

## How to Use This File

1. Read all project inputs (product.md, tech-stack.md, user answers to gap questions)
2. Extract architectural signals — phrases, requirements, or characteristics that imply specific patterns
3. Match signals to patterns below. Each pattern lists its trigger signals.
4. Assign each matched pattern to a tier:
   - **Strongly recommended** — multiple strong signal matches; system needs these
   - **Recommended** — single strong signal match; will save pain later
   - **Consider for later** — inferred signals; may emerge during implementation
5. Present recommendations to developer with trade-offs for accept/reject/modify

Typical tier assignment is noted per pattern, but the actual tier depends on signal strength in the specific project.

---

## 1. Saga (Orchestration)

**Signals:**
- "workflow spans multiple services"
- "rollback on failure" / "compensating transactions"
- "multi-step business process" with "consistency across services"
- "long-running transaction" / "distributed transaction"
- "if step N fails, undo steps 1..N-1"

**When to use:**
- Business workflows that touch 2+ services and require all-or-nothing semantics
- When you need visibility into workflow state (which step succeeded, which failed)
- When compensating actions are well-defined (cancel order, release inventory, refund payment)

**When NOT to use:**
- Single-service operations — use a database transaction instead
- Fire-and-forget events where rollback is meaningless
- When all operations can run in a single database transaction

**Trade-offs:**
- (+) Clear failure handling and compensation logic
- (+) Workflow state is observable and debuggable
- (-) Complexity: each step needs a compensating action
- (-) Eventual consistency — intermediate states are visible to other services
- (-) Orchestrator can become a bottleneck / single point of failure

**Implementation notes:**
- Orchestration (central coordinator) preferred over choreography (event chain) for >3 steps
- Consider Temporal/Durable Execution frameworks for complex sagas
- Each saga step should be idempotent

**Typical tier:** Strongly recommended (when signals match)

---

## 2. Transactional Outbox

**Signals:**
- "publish events after database writes"
- "event-driven" + "database"
- "guarantee event delivery"
- "at-least-once delivery"
- "domain events" / "event publishing"
- "no lost events" / "reliable messaging"

**When to use:**
- Any service that writes to a database AND publishes events/messages
- When you cannot afford lost events (financial transactions, order processing)
- When using a message broker (Kafka, RabbitMQ, Redis Streams)

**When NOT to use:**
- Read-only services that don't publish events
- When events are purely informational and losing one is acceptable
- In-memory or stateless services without database writes

**Trade-offs:**
- (+) Atomic: event and state change happen in the same DB transaction
- (+) No distributed transaction needed (no 2PC)
- (+) Works with any message broker
- (-) Requires a polling mechanism or CDC (Change Data Capture) to read the outbox
- (-) Adds an outbox table and a relay/poller component
- (-) At-least-once, not exactly-once — consumers need idempotency

**Implementation notes:**
- Outbox table: id, aggregate_type, aggregate_id, event_type, payload, created_at, published_at
- Relay options: polling (simple), CDC with Debezium (robust), transactional log tailing
- Keep events small; include only IDs and changed fields

**Typical tier:** Strongly recommended (near-universal for event-driven systems)

---

## 3. CQRS (Command Query Responsibility Segregation)

**Signals:**
- "read-heavy" / "dashboard" / "reporting"
- "write model different from read model"
- "complex queries across multiple aggregates"
- "search" + "filter" + "sort" on denormalized data
- "analytics" / "real-time dashboard"
- "query performance" concerns

**When to use:**
- Read patterns differ significantly from write patterns
- Complex query requirements (multi-field search, faceted filtering, aggregations)
- When you want to scale reads independently from writes
- Dashboard or reporting services with heavy query loads

**When NOT to use:**
- Simple CRUD with similar read/write patterns
- Small data volumes where a single model performs fine
- When the added complexity of two models isn't justified by performance needs

**Trade-offs:**
- (+) Optimize read and write models independently
- (+) Read model can be denormalized for query performance
- (+) Scale reads (replicas, caching, search indices) without affecting writes
- (-) Eventual consistency between write and read models
- (-) Two models to maintain and keep in sync
- (-) Added infrastructure (projections, event handlers, read store)

**Implementation notes:**
- Start simple: same DB with a materialized view or denormalized read table
- Graduate to separate read store (Elasticsearch, Redis) if query patterns demand it
- Often paired with Event Sourcing, but can be used independently

**Typical tier:** Consider for later (unless dashboard/reporting is a core requirement)
**Deferred trigger:** If dashboard or list queries exceed 500ms consistently

---

## 4. Circuit Breaker

**Signals:**
- "external API calls" / "third-party integrations"
- "unreliable dependencies" / "flaky service"
- "timeout" + "retry" + "failure handling"
- "graceful degradation when X is down"
- "upstream service outage shouldn't take us down"

**When to use:**
- Any call to an external or third-party service
- Inter-service calls in a microservice architecture
- When a dependency failure could cascade to your service

**When NOT to use:**
- In-process function calls
- Database calls (use connection pooling and timeouts instead)
- When there's no meaningful fallback behavior

**Trade-offs:**
- (+) Prevents cascade failures
- (+) Fails fast when dependency is known-down (saves timeout waits)
- (+) Self-healing: automatically retries after cooldown
- (-) Added complexity in call paths
- (-) Need to define fallback behavior for each dependency
- (-) State management (open/half-open/closed) adds operational overhead

**Implementation notes:**
- Three states: Closed (normal), Open (failing fast), Half-Open (testing recovery)
- Configure per dependency: failure threshold, timeout, cooldown period
- Libraries: resilience4j (Java), Polly (.NET), tenacity (Python), cockatiel (Node)

**Typical tier:** Recommended (any project with external dependencies)
**Deferred trigger:** If external API failure rate exceeds 5% over a 15-minute window

---

## 5. Event Sourcing

**Signals:**
- "audit trail" / "full history of changes"
- "regulatory compliance" / "immutable record"
- "time travel" / "reconstruct state at any point"
- "undo/redo" functionality
- "complex domain" + "behavior-rich entities"
- "domain events are the source of truth"

**When to use:**
- Audit/compliance requirements where you must prove what happened and when
- Financial systems where every state change must be recorded
- Complex domains where capturing intent (events) is more valuable than snapshots

**When NOT to use:**
- Simple CRUD applications without audit requirements
- When the read model is the primary concern and event replay is unnecessary
- High-write-volume systems where event storage costs become prohibitive (without compaction)

**Trade-offs:**
- (+) Complete audit trail for free
- (+) Can reconstruct any past state
- (+) Events capture intent, not just final state
- (+) Natural fit with CQRS and event-driven architectures
- (-) Significant complexity increase
- (-) Event schema evolution is hard (versioning, upcasting)
- (-) Aggregate replay can be slow without snapshots
- (-) Developers must think in events, not state mutations

**Implementation notes:**
- Event store: append-only. Never delete or update events.
- Snapshots: periodic state snapshots to avoid replaying from event 0
- Schema evolution: event upcasters transform old events to new schema on read
- Often combined with CQRS for the read side

**Typical tier:** Consider for later (unless audit/compliance is a day-1 requirement)
**Deferred trigger:** If audit or compliance requirements emerge during implementation

---

## 6. API Gateway

**Signals:**
- "multiple backend services" + "single entry point"
- "mobile and web clients" with different data needs
- "rate limiting" / "authentication at the edge"
- "API versioning" across services
- "request routing" / "load balancing" at the application layer

**When to use:**
- Multiple backend services that clients need to access
- When you need centralized cross-cutting (auth, rate limiting, logging) for all APIs
- When different clients (mobile, web, internal) access the same services

**When NOT to use:**
- Single-service architectures (the service IS the API)
- Internal-only services that don't face external clients
- When a simple reverse proxy (nginx) covers your routing needs

**Trade-offs:**
- (+) Single entry point simplifies client integration
- (+) Centralizes auth, rate limiting, CORS, logging
- (+) Can aggregate responses from multiple services
- (-) Single point of failure (mitigate with HA deployment)
- (-) Added latency on every request (one extra hop)
- (-) Can become a deployment bottleneck if tightly coupled to services

**Implementation notes:**
- Off-the-shelf: Kong, AWS API Gateway, Envoy, Traefik
- Build-your-own only if you have very specific routing/transformation needs
- Keep the gateway thin: route + cross-cutting only, no business logic

**Typical tier:** Recommended (any multi-service project with external clients)

---

## 7. Backend for Frontend (BFF)

**Signals:**
- "mobile app" + "web app" with "different data needs"
- "API returns too much data for mobile" / "over-fetching"
- "each client needs its own aggregation"
- "GraphQL for web, REST for mobile" or similar divergence
- "client-specific transformations"

**When to use:**
- Two or more client types (web, mobile, TV, partner API) with divergent needs
- When a single API leads to over-fetching or under-fetching for specific clients
- When client teams want autonomy over their backend-for-frontend layer

**When NOT to use:**
- Single client type (just build a good API)
- When a shared API with field selection (GraphQL, sparse fields) solves the problem
- Small teams where maintaining multiple BFFs is more burden than benefit

**Trade-offs:**
- (+) Each client gets exactly the data/format it needs
- (+) Client teams can iterate on their BFF independently
- (+) Insulates clients from backend service changes
- (-) Multiple services to maintain (one per client type)
- (-) Business logic can leak into BFFs if boundaries aren't enforced
- (-) Duplication of similar logic across BFFs

**Implementation notes:**
- One BFF per client type, not per page or feature
- BFF should only aggregate and transform, never contain core business logic
- Consider GraphQL as an alternative to BFF if the divergence is about field selection

**Typical tier:** Consider for later (unless multi-platform is a core requirement)
**Deferred trigger:** If mobile client team reports over-fetching or needing 3+ chained API calls per screen

---

## 8. Strangler Fig

**Signals:**
- "legacy system" / "monolith" that needs "migration" / "modernization"
- "can't rewrite all at once"
- "gradual migration" / "incremental replacement"
- "legacy and new system run in parallel"
- "existing system" + "new architecture"

**When to use:**
- Migrating from monolith to microservices (or any major architecture change)
- When a full rewrite is too risky or too long
- When you need the old and new systems to coexist during transition

**When NOT to use:**
- Greenfield projects with no legacy system
- When the legacy system is small enough for a complete rewrite
- When there's no clear domain boundary to strangle first

**Trade-offs:**
- (+) Incremental, low-risk migration
- (+) Old system remains operational throughout
- (+) Each strangled piece validates the approach before proceeding
- (-) Maintaining two systems simultaneously
- (-) Routing layer (facade) adds complexity
- (-) Can stall: "temporary" routing becomes permanent if migration loses momentum

**Implementation notes:**
- Start with the most isolated domain boundary
- Use a facade/proxy that routes traffic to old or new system per feature
- Track percentage of traffic flowing to new system — make progress visible

**Typical tier:** Strongly recommended (when legacy signals present)

---

## 9. Sidecar / Ambassador

**Signals:**
- "consistent cross-cutting across services in different languages"
- "service mesh" / "Istio" / "Envoy"
- "polyglot services" + "shared infrastructure concerns"
- "mTLS" / "service-to-service encryption"
- "observability without modifying application code"

**When to use:**
- Multi-language microservices that need uniform cross-cutting behavior
- When infrastructure concerns (TLS, tracing, retries) shouldn't be in application code
- When adopting a service mesh (Istio, Linkerd)

**When NOT to use:**
- Single-language services where a shared library achieves the same thing
- Monolithic applications
- When the operational overhead of sidecars exceeds the benefit

**Trade-offs:**
- (+) Language-agnostic: one sidecar works with any service
- (+) Separates infrastructure from business logic cleanly
- (+) Can add capabilities (mTLS, retries, tracing) without touching application code
- (-) Resource overhead: each service gets an extra container
- (-) Increased latency (traffic proxied through sidecar)
- (-) Operational complexity: sidecar upgrades affect all services

**Implementation notes:**
- Service mesh (Istio/Linkerd) provides sidecars automatically
- For simpler needs, Envoy as a standalone sidecar handles TLS + retries + observability
- Only adopt if you have 3+ services in different languages, otherwise a shared library is simpler

**Typical tier:** Consider for later (unless polyglot microservices from day 1)

---

## 10. Bulkhead

**Signals:**
- "isolate failures" / "blast radius"
- "one slow endpoint shouldn't affect others"
- "resource exhaustion" in "thread pool" or "connection pool"
- "noisy neighbor" problem
- "multi-tenant" + "isolation"

**When to use:**
- When failure in one subsystem could exhaust shared resources (threads, connections, memory)
- Multi-tenant systems where one tenant's traffic shouldn't impact others
- Services with multiple downstream dependencies of varying reliability

**When NOT to use:**
- Single-purpose services with one dependency
- When resource constraints are already handled at the infrastructure level (container limits)
- When the overhead of maintaining separate pools isn't justified

**Trade-offs:**
- (+) Contains failures to their compartment
- (+) Critical paths remain responsive even when non-critical ones fail
- (+) Effective in multi-tenant environments
- (-) Underutilization: dedicated pools may sit idle while others are saturated
- (-) Configuration complexity: sizing each pool correctly
- (-) More pools = more monitoring

**Implementation notes:**
- Thread pool isolation: separate pools for different downstream services
- Connection pool isolation: dedicated DB/HTTP connection pools per dependency
- Semaphore isolation: lightweight alternative to thread pools
- Often combined with circuit breaker for comprehensive resilience

**Typical tier:** Recommended (any system with 2+ external dependencies)

---

## 11. Retry with Exponential Backoff + Jitter

**Signals:**
- "transient failures" / "temporary errors"
- "network issues" / "timeout"
- "retry" + "not hammer the service"
- "idempotent operations"
- "429 Too Many Requests" / "rate limited"

**When to use:**
- Any remote call (HTTP, gRPC, message queue) that can fail transiently
- When the operation is idempotent (safe to retry)
- Rate-limited APIs where you need to back off gracefully

**When NOT to use:**
- Non-idempotent operations (unless you add idempotency keys)
- Validation errors or business logic failures (4xx that won't change on retry)
- When immediate failure reporting is preferred over delayed success

**Trade-offs:**
- (+) Handles transient failures transparently
- (+) Jitter prevents thundering herd on recovery
- (+) Simple to implement with existing libraries
- (-) Increases latency during failure (retry delays)
- (-) Can mask underlying problems if retries always succeed
- (-) Without a circuit breaker, retries can worsen an overloaded service

**Implementation notes:**
- Formula: `delay = min(base * 2^attempt + random_jitter, max_delay)`
- Typical: base=100ms, max_delay=30s, max_attempts=3-5
- Always pair with a circuit breaker for external dependencies
- Log retries at WARN level to surface flaky dependencies

**Typical tier:** Strongly recommended (near-universal for distributed systems)

---

## 12. Distributed Tracing

**Signals:**
- "multiple services" + "debugging production issues"
- "where is the latency?" / "which service is slow?"
- "trace a request across services"
- "observability" / "OpenTelemetry"
- "correlate logs across services"

**When to use:**
- Any multi-service system where requests span 2+ services
- When production debugging requires understanding the full call chain
- When you need to identify latency bottlenecks across service boundaries

**When NOT to use:**
- Single-service applications (structured logging with request IDs is sufficient)
- When the operational overhead of a tracing backend isn't justified by system complexity

**Trade-offs:**
- (+) End-to-end visibility into request flow
- (+) Latency breakdown per service/operation
- (+) Correlates logs, metrics, and traces via trace ID
- (-) Requires instrumentation in every service
- (-) Tracing backend (Jaeger, Tempo, Zipkin) needs hosting and storage
- (-) High-volume systems need sampling to control costs

**Implementation notes:**
- OpenTelemetry is the standard: vendor-neutral, wide language support
- Auto-instrumentation for HTTP/gRPC frameworks reduces manual work
- Propagate trace context via W3C Trace Context headers
- Sample at 1-10% in production for cost control; 100% in staging

**Typical tier:** Strongly recommended (any multi-service system)

---

## 13. Feature Flags

**Signals:**
- "gradual rollout" / "canary release"
- "A/B testing"
- "kill switch" / "turn off feature without deploy"
- "dark launch" / "test in production"
- "decouple deploy from release"

**When to use:**
- When you want to deploy code without exposing it to all users
- Gradual rollouts: 1% -> 10% -> 50% -> 100%
- When features need an emergency kill switch
- When product teams want to A/B test behavior

**When NOT to use:**
- Infrastructure changes that can't meaningfully be flagged
- When flag management overhead exceeds the risk of direct deployment
- As a permanent configuration mechanism (flags should be short-lived)

**Trade-offs:**
- (+) Deploy and release are decoupled — ship code any time
- (+) Instant rollback without redeployment
- (+) Enables trunk-based development
- (-) Flag debt: old flags accumulate if not cleaned up
- (-) Testing combinatorics: N flags = 2^N possible states
- (-) Flag evaluation adds latency if using a remote service

**Implementation notes:**
- Start simple: environment variable or config file flags
- Graduate to a flag service (LaunchDarkly, Unleash, Flipt) if you need targeting rules
- Every flag should have an owner and an expiry date
- Track active flags; alert when count exceeds threshold

**Typical tier:** Recommended (any team doing continuous deployment)

---

## 14. Blue-Green / Canary Deployment

**Signals:**
- "zero-downtime deployment"
- "rollback instantly" / "instant rollback"
- "test in production before full rollout"
- "deployment safety" / "reduce deployment risk"
- "two environments" / "swap traffic"

**When to use:**
- Production systems where downtime during deploys is unacceptable
- When you need verified rollback capability
- When deployments are frequent (daily+)

**When NOT to use:**
- Early-stage projects where downtime during deploy is acceptable
- When infrastructure costs of maintaining two environments aren't justified
- Database schema changes that can't run in both versions simultaneously

**Trade-offs:**
- Blue-Green:
  - (+) Simple: swap all traffic between identical environments
  - (+) Instant, reliable rollback (swap back)
  - (-) Double infrastructure cost during deployment
  - (-) Database migrations must be backward-compatible
- Canary:
  - (+) Gradual: route a percentage of traffic to the new version
  - (+) Detect issues with minimal user impact
  - (-) More complex routing infrastructure
  - (-) Longer deployment windows

**Implementation notes:**
- Blue-Green: two identical environments, load balancer swaps traffic
- Canary: weighted routing (1% -> 5% -> 25% -> 100%) with health monitoring
- Both require backward-compatible database migrations (expand-and-contract pattern)
- Container orchestrators (Kubernetes) have built-in support for both

**Typical tier:** Consider for later (unless deployment frequency is high from day 1)
**Deferred trigger:** If deploying more than once per day and rollback is needed

---

## 15. Database per Service

**Signals:**
- "microservices" + "data isolation"
- "each service owns its data"
- "no shared database"
- "service autonomy" / "independent deployment"
- "polyglot persistence" (different DBs for different services)

**When to use:**
- Microservice architectures where services must be independently deployable
- When different services have fundamentally different data storage needs
- When a shared database creates coupling between teams

**When NOT to use:**
- Monolithic applications (a single database is fine)
- Small teams (< 3 developers) where the overhead of multiple databases isn't warranted
- When cross-service queries are frequent and a shared DB is significantly simpler

**Trade-offs:**
- (+) Services are truly independent — schema changes don't cascade
- (+) Each service picks the best storage for its needs (Postgres, Redis, Elasticsearch)
- (+) Independent scaling of data tier per service
- (-) Cross-service queries require API composition or data replication
- (-) Distributed transactions are hard (use sagas instead)
- (-) More databases to operate, back up, monitor

**Implementation notes:**
- Start with logical isolation (separate schemas in one DB) if full separation isn't needed yet
- Use events (via outbox) to replicate needed data between services
- Cross-service joins become API calls or materialized views in a read store

**Typical tier:** Recommended (any microservice project; start with logical isolation)

---

## 16. Rate Limiting / Throttling

**Signals:**
- "protect API from abuse"
- "fair usage" / "quotas"
- "multi-tenant" + "noisy neighbor"
- "DDoS protection" / "load shedding"
- "public API" / "third-party consumers"

**When to use:**
- Any public-facing API
- Multi-tenant systems where fair resource sharing matters
- When backend services could be overwhelmed by burst traffic

**When NOT to use:**
- Internal-only APIs with trusted callers and known capacity
- When infrastructure-level rate limiting (WAF, API gateway) already covers the need

**Trade-offs:**
- (+) Protects backend from overload
- (+) Ensures fair usage in multi-tenant systems
- (+) Returns meaningful 429 errors with Retry-After headers
- (-) Complexity in choosing the right algorithm and limits
- (-) Distributed rate limiting (across instances) requires shared state (Redis)
- (-) Can frustrate legitimate users if limits are too aggressive

**Implementation notes:**
- Algorithms: Token bucket (burst-friendly), sliding window (smooth), fixed window (simple)
- Per-user, per-API-key, or per-IP depending on context
- Use Redis for distributed rate limiting across service instances
- Return `Retry-After` header in 429 responses

**Typical tier:** Recommended (any user-facing API)

---

## 17. Health Checks (Liveness + Readiness)

**Signals:**
- "container orchestration" / "Kubernetes"
- "service health" / "monitoring"
- "graceful startup" / "dependency warmup"
- "load balancer" + "health endpoint"

**When to use:**
- Any service running in a container orchestrator or behind a load balancer
- When you need automated restart on failure (liveness) or traffic removal during startup (readiness)

**When NOT to use:**
- Never skip this. Every networked service needs health checks.

**Trade-offs:**
- (+) Automated recovery: orchestrator restarts unhealthy services
- (+) Graceful startup: traffic only flows when service is ready
- (+) Standard practice: every ops tool expects /healthz and /readyz
- (-) Minimal: liveness check must not depend on external services (prevents restart loops)

**Implementation notes:**
- `/healthz` (liveness): "am I alive?" — should check internal state only, NOT dependencies
- `/readyz` (readiness): "can I serve traffic?" — check database, cache, queue connectivity
- Keep liveness checks fast (< 100ms) and dependency-free
- Readiness checks can include dependency connectivity

**Typical tier:** Strongly recommended (always; effectively mandatory)

---

## 18. Graceful Shutdown

**Signals:**
- "zero-downtime" / "rolling deployment"
- "in-flight requests" / "drain connections"
- "SIGTERM handling"
- "container" / "Kubernetes" + "deployment"

**When to use:**
- Any long-lived service that handles requests or processes messages
- Container orchestrated services that restart during deployments
- Services with in-flight work that shouldn't be dropped

**When NOT to use:**
- Short-lived batch jobs that are inherently restartable
- Services where dropping in-flight work is acceptable

**Trade-offs:**
- (+) No dropped requests during deployments
- (+) Background tasks complete cleanly
- (+) Database connections released properly
- (-) Shutdown timeout must be configured (too short = forced kill, too long = slow deploys)

**Implementation notes:**
- Catch SIGTERM, stop accepting new work, finish in-flight work, close connections, exit
- Kubernetes: set `terminationGracePeriodSeconds` (default 30s)
- Close consumers (message queues) before closing producers (outbox relay)
- Log shutdown steps for debugging deployment issues

**Typical tier:** Strongly recommended (always; pair with health checks)

---

## Quick Reference: Signal Lookup

| If you see this signal... | Check these patterns |
|---|---|
| "workflow" + "multiple services" | Saga, Outbox |
| "events" + "database writes" | Outbox |
| "read-heavy" / "dashboard" | CQRS |
| "external API" / "unreliable" | Circuit Breaker, Retry, Bulkhead |
| "audit trail" / "compliance" | Event Sourcing |
| "multiple clients" (mobile+web) | API Gateway, BFF |
| "legacy migration" | Strangler Fig |
| "polyglot" / "service mesh" | Sidecar |
| "noisy neighbor" / "isolation" | Bulkhead, Rate Limiting |
| "transient failures" | Retry with Backoff |
| "debugging across services" | Distributed Tracing |
| "gradual rollout" | Feature Flags, Blue-Green/Canary |
| "zero downtime deploy" | Blue-Green/Canary, Graceful Shutdown |
| "microservices" + "data" | Database per Service, Outbox |
| "public API" / "abuse" | Rate Limiting, API Gateway |
| "health" / "container" | Health Checks, Graceful Shutdown |
