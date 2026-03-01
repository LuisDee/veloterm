# Hook 02: Interface Verification

**When:** Before implementing code that consumes another track's API or events.

## Purpose

Verify that the interface contract in `architect/interfaces.md` matches what the producing track actually provides. Catch contract drift early, before you build against a wrong assumption.

## Procedure

### 1. Identify the interface you need

Note: which track owns it, what endpoint/event you need, what you expect it to return.

### 2. Read the contract

Open `architect/interfaces.md` and find the section for the owning track. Note the expected:
- Endpoint path, method, request/response schema
- OR event name, payload schema, publishing trigger

### 3. Check the producer track's state

Open `conductor/tracks/<producer_track>/metadata.json` and check `state`.

### If producer is COMPLETE:

**Tier 1 — Integration tests exist:**
Check if the producer track has integration tests that exercise the endpoint/event:
- Look in the producer's test directory for API tests or event handler tests
- If tests exist and cover the contract → **trust the contract**. Implement against interfaces.md.

**Tier 2 — No integration tests, verify implementation:**
If no tests cover the contract, verify the implementation directly:
- Read the producer's actual code: route definitions, response schemas, event payloads
- Compare against interfaces.md
- If they match → **proceed with confidence**
- If they don't match → **log INTERFACE_MISMATCH discovery** (BLOCKING urgency)

### If producer is IN_PROGRESS:

The producer hasn't finished yet. The contract in interfaces.md is the intended design.

- Implement against the interfaces.md contract
- Use **mocks or stubs** for the actual calls
- Add a TODO comment: `# TODO: Validate against real endpoint when <producer_track> is complete`
- This is expected and normal — the wave system is designed for this

### If producer is NOT_STARTED:

The producer doesn't exist yet.

- Implement against the interfaces.md contract using **mocks**
- Add a TODO comment: `# TODO: Replace mock with real call when <producer_track> is available`
- If the interface contract seems incomplete or wrong, log a NEW_DEPENDENCY or INTERFACE_MISMATCH discovery

### 4. Document what you did

In your implementation, add a brief comment noting which verification tier you used:
```python
# Interface: /v1/auth/me (Track 03_auth) — verified against implementation (Tier 2)
```
or
```python
# Interface: /v1/resources (Track 04_api_core) — mocked, track IN_PROGRESS (Tier 3)
```

## When to Log Discoveries

- **INTERFACE_MISMATCH**: The implementation doesn't match interfaces.md (field names differ, response structure wrong, missing fields). Always BLOCKING if you're actively consuming it.
- **NEW_DEPENDENCY**: You need an endpoint/event that isn't in any track's interfaces. The capability is missing entirely.
- **TRACK_EXTENSION**: The endpoint exists but needs a small addition (e.g., needs a filter parameter that isn't there yet). < 5 tasks.
