# Cross-Cutting Concerns Catalog

Evaluate every item in this catalog during `/architect-decompose` Step 3, regardless of what the developer mentions. Items not applicable to the project are marked "N/A" with a one-line reason. Items accepted become constraints in `architect/cross-cutting.md` v1.

## How to Use This File

1. Walk through each category based on project characteristics
2. For each item, determine: applicable? If yes, what specific convention?
3. Present applicable items to the developer alongside architecture pattern recommendations
4. Accepted items become versioned constraints in cross-cutting.md

---

## Always Evaluate

These apply to every project. Skip only with explicit justification.

### Structured Logging
Define a logging library and format for every service. Structured (JSON) logs enable machine parsing, aggregation, and correlation. Without this, debugging in production requires grepping raw text across multiple services.

### Error Handling Convention
Standardize error response format (e.g., RFC 7807 Problem Details) and internal error propagation. Inconsistent error handling leads to fragile client code and poor developer experience. Define: response format, error codes, what gets logged vs. returned, and whether stack traces appear in non-production environments.

### Health Checks (Liveness + Readiness)
Every service exposes `/healthz` (am I alive — internal state only) and `/readyz` (can I serve traffic — includes dependency checks). Required for container orchestration, load balancer integration, and automated recovery. Liveness must never check external dependencies to prevent restart cascades.

### Configuration Management
Define how services access configuration: environment variables, config files, secrets manager, or a combination. Separate secrets from non-secrets. Document the strategy once so every track follows it. Without this, each track invents its own approach and secrets end up in config files.

### Graceful Shutdown
Every long-lived service handles SIGTERM by stopping new work intake, completing in-flight requests, closing connections, and exiting. Required for zero-downtime deployments and clean container orchestration. Define the shutdown sequence and timeout.

### Input Validation
Define where and how input is validated: at the API boundary, in the domain layer, or both. Specify the validation library/approach. Without this, validation is inconsistent — some endpoints validate, others trust input, and security holes open up.

### Database Connection Pooling
Configure connection pools for every service that accesses a database. Define pool size guidelines, connection timeout, idle timeout, and health check queries. Without pooling, services under load exhaust database connections and cascade-fail.

### Timeout Policies
Set explicit timeouts on every external call (HTTP, gRPC, database queries, cache operations). Define default timeouts and how they can be overridden per-call. Without timeouts, one slow dependency blocks threads indefinitely and cascades failure to callers.

---

## If Multi-Service

Evaluate when the architecture has 2+ independently deployed services.

### Distributed Tracing
Instrument all services with OpenTelemetry (or equivalent) for correlated traces, metrics, and logs. Without tracing, debugging a request that spans multiple services means correlating logs by timestamp and guessing. Define: trace propagation format (W3C Trace Context), sampling rate, and tracing backend.

### Service Discovery
Define how services find each other: DNS-based (Kubernetes Services), config-based (environment variables), or registry-based (Consul, etcd). Without this, service URLs are hardcoded, and environment changes break connections.

### API Versioning
Define how APIs evolve without breaking consumers: URL versioning (/v1/), header versioning, or content negotiation. Choose one approach and apply it consistently. Without this, breaking changes propagate silently and consumers fail unpredictably.

### Event Schema Versioning
Define how event schemas evolve: schema registry, embedded version field, or consumer-driven contracts. Producers and consumers must agree on how to handle schema changes. Without this, a field rename breaks every consumer.

### Idempotency for Message Handlers
Every message consumer must be idempotent — processing the same message twice produces the same result. Define the idempotency strategy: idempotency keys, deduplication tables, or natural idempotency. Without this, at-least-once delivery causes duplicate processing.

---

## If User-Facing

Evaluate when the system has end users (web, mobile, or API consumers).

### Authentication + Authorization
Define the auth strategy: session-based, JWT, OAuth2/OIDC, API keys, or a combination. Specify: where tokens are issued, how they're validated, how permissions are modeled (RBAC, ABAC), and where auth middleware runs. Without this, each service implements auth differently, creating security gaps.

### CORS Policy
Define which origins, methods, and headers are allowed for browser-based clients. Be specific — `Access-Control-Allow-Origin: *` is almost never correct for authenticated APIs. Define the policy centrally (API gateway or shared middleware) rather than per-service.

### Session Management
If using sessions: define storage (Redis, database, cookie-only), expiry, renewal, and invalidation. Define what constitutes a session versus a stateless token. Without this, sessions leak, don't expire, or lose state across service instances.

---

## If Data-Heavy

Evaluate when the system manages significant persistent data or has compliance requirements.

### Backup and Recovery
Define backup strategy: frequency, retention period, storage location, and recovery testing cadence. Include both database backups and any file/blob storage. Without this, data loss from hardware failure, human error, or corruption is permanent.

### Data Retention Policy
Define how long each category of data is kept and when it's purged. Regulatory requirements (GDPR, HIPAA, SOX) often mandate both minimum retention and maximum retention. Without this, data grows unbounded and compliance audits fail.

### PII Handling
Identify which fields contain Personally Identifiable Information. Define: encryption at rest, encryption in transit, access logging, anonymization for non-production environments, and right-to-deletion procedures. Without this, PII leaks into logs, test data, analytics, and backups.

### Migration Strategy
Define how database schema changes are applied: versioned migrations (Flyway, Alembic, Knex), declarative schemas, or manual SQL. Define the backward-compatibility requirement: can the previous application version run against the new schema? Without this, deployments corrupt data or cause downtime.
