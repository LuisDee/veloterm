# Hook 04: Phase Validation

**When:** Before marking a phase complete in plan.md.

## Purpose

Verify that all work done in this phase complies with the current cross-cutting constraints. Catch violations before they accumulate across phases.

## Procedure

### 1. Read current constraints

Open `architect/cross-cutting.md` and read ALL constraint sections up to and including the version noted in your track's `metadata.json` `cc_version_current`.

### 2. Check each applicable constraint

Walk through every constraint and verify compliance for the work done in THIS phase. Common checks:

**Observability:**
- [ ] New endpoints have OpenTelemetry instrumentation (traces)
- [ ] Structured logging on key operations (not just print/console.log)
- [ ] Health check endpoints exist if this phase added a new service

**Error Handling:**
- [ ] Error responses follow the defined format (e.g., RFC 7807)
- [ ] Errors are logged with trace ID for correlation
- [ ] No stack traces returned in production error responses

**Authentication / Authorization:**
- [ ] Protected endpoints have auth middleware applied
- [ ] Authorization checks match the defined permission model

**Transactional Outbox (if this track publishes events):**
- [ ] Events published through the outbox table, not directly to broker
- [ ] Event and state change are in the same database transaction

**Testing:**
- [ ] Tests written for new functionality (TDD if specified)
- [ ] Code coverage meets the defined threshold
- [ ] Integration tests for API boundaries (if this phase added endpoints)

**API Conventions (if this phase added endpoints):**
- [ ] Response format matches the defined envelope structure
- [ ] Pagination follows the defined approach (cursor-based, etc.)
- [ ] Field naming conventions followed (snake_case, camelCase, etc.)

**Additional project-specific constraints:**
- [ ] Check any constraints added in CC versions after v1

### 3. If any check fails

Fix the violation before marking the phase complete. These are the project's own standards — skipping them creates debt that compounds.

### 4. If a constraint itself is wrong

If you discover that a constraint is impractical, contradictory, or harmful:
- Do NOT silently skip it
- Log a CROSS_CUTTING_CHANGE discovery (via Hook 03) explaining why the constraint should change
- For this phase, apply the constraint as-is (or note the exception with a TODO)
- The discovery will be processed during the next sync

### 5. Mark phase complete

After all checks pass:
1. Check off all task checkboxes in plan.md for this phase
2. Check off the phase validation checkbox
3. The Conductor manual verification checkpoint is handled by Conductor, not Architect
