# Hook 03: Discovery Check

**When:** After completing each task during implementation.

## Purpose

Identify emergent work that wasn't captured in the original decomposition. Discoveries are how the architecture stays alive — they capture reality as it diverges from the plan.

## Procedure

### 1. Assess

After completing a task, briefly consider:

- Did this task reveal **assumptions that don't hold**? (Expected an API to work one way, but it doesn't)
- Is there **functionality missing** from any planned track? (Needed a feature nobody planned for)
- Are there **uncaptured dependencies**? (This track needs something from another track that isn't in the DAG)
- Should a **cross-cutting concern change**? (Discovered that all services need a behavior nobody specified)
- Did a **deferred pattern trigger** fire? (See Section 6 below)

If the answer to ALL questions is NO, continue to the next task. Don't overthink this — most tasks will have no discoveries.

### 2. If YES — Write a discovery file

Create a file in `architect/discovery/pending/` with the format:

**Filename:** `{track_id}-{ISO-timestamp}-{6-char-random-hex}.md`
Example: `track-04-2026-02-08T14-30-00Z-a3f2b8.md`

**Contents:**
```markdown
## Discovery
- **Source:** Track <track_id>, Phase <N>, Task <N.M>
- **Timestamp:** <ISO-8601>
- **Discovery:** <What you found. Be specific.>
- **Classification:** <See decision tree below>
- **Suggested scope:** <track_name — brief description of what needs to happen>
- **Dependencies:** <What this depends on, what it partially blocks>
- **Urgency:** <BLOCKING | NEXT_WAVE | BACKLOG>
```

### 3. Classify using the decision tree

```
Q1: Does this affect multiple tracks or the whole system?
│
├─ YES ─┐
│       Is it a behavioral rule (HOW things behave)?
│       ├─ YES → CROSS_CUTTING_CHANGE
│       └─ NO  → ARCHITECTURE_CHANGE
│
└─ NO (one track / small area) ─┐
        │
        Q2: Does it belong in an existing track?
        ├─ YES: < 5 tasks, same tech? → TRACK_EXTENSION
        │       5+ tasks or diff tech? → NEW_TRACK
        │
        └─ NO: Need another track's output? → NEW_DEPENDENCY
               API/event contract wrong?   → INTERFACE_MISMATCH
               Entirely new functionality?  → NEW_TRACK
```

### 4. Assess urgency

- **BLOCKING:** Cannot continue current work without resolution. Or: a dependent track in the same wave is stuck.
- **NEXT_WAVE:** Not blocking now, but must be resolved before the next wave starts.
- **BACKLOG:** Nice to have. No track is blocked.

### 5. Continue working

After writing the discovery file, **continue with your current task**. Do NOT scope-creep into addressing the discovery yourself. The discovery will be processed during the next `/architect-sync`.

Exception: If urgency is BLOCKING and you literally cannot proceed, notify the developer immediately.

---

## 6. Deferred Pattern Triggers

During architecture research, some patterns were classified as "Consider for Later" with measurable trigger conditions. Check these triggers as you work:

<!-- TRIGGERS INJECTED BY /architect-decompose — DO NOT EDIT ABOVE THIS LINE -->

<!-- Default triggers (replaced with project-specific ones during decompose): -->

| Pattern | Trigger Condition | Discovery Classification |
|---------|-------------------|--------------------------|
| CQRS | Dashboard or list queries consistently exceed 500ms | NEW_TRACK |
| Circuit Breaker | External API failure rate exceeds 5% over a 15-minute window | CROSS_CUTTING_CHANGE |
| Event Sourcing | Audit or compliance requirements emerge (e.g., "we need to prove what happened") | ARCHITECTURE_CHANGE |
| BFF (Backend for Frontend) | Mobile client needs 3+ chained API calls per screen, or significant over-fetching | NEW_TRACK |
| Blue-Green Deployment | Deploying more than once per day and rollback capability is needed | CROSS_CUTTING_CHANGE |

<!-- END TRIGGERS -->

If a trigger condition is met, log a discovery with the indicated classification. Include the measurement that triggered it (e.g., "dashboard query P95 is 720ms, exceeding 500ms threshold").
