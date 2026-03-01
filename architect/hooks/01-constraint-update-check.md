# Hook 01: Constraint Update Check

**When:** Before starting ANY new phase during implementation.

## Purpose

Detect whether cross-cutting constraints have changed since this track started (or since the last phase). If they have, adopt the new constraints for remaining phases without reworking completed work.

## Procedure

### 1. Read your track's CC version

Open `conductor/tracks/<your_track>/metadata.json` and note:
- `cc_version_at_start` — the version when this track began
- `cc_version_current` — the last version this track adopted

### 2. Read the current CC version

Open `architect/cross-cutting.md` and find the latest version heading (e.g., `## v1.2`).

### 3. Compare versions

**If versions match:** No changes. Proceed with the phase.

**If the current CC version is newer than cc_version_current:**

a. Read the new constraint sections added since your version. Look for headings between your version and the current version.

b. For each new constraint, determine if it applies to this track:
   - Check the "Applies to" field
   - If it says "ALL" or mentions your track, it applies

c. For applicable constraints:
   - Apply them to REMAINING phases only
   - Do NOT rework already-completed phases
   - If rework IS genuinely needed on completed phases, log a TRACK_EXTENSION discovery (see Hook 03) with patch tasks for the completed phases

d. Update metadata.json:
   - Set `cc_version_current` to the latest CC version

### 4. Resume phase

Continue with the phase, incorporating any new constraints into your work.

## Example

```
metadata.json says: cc_version_current = "v1"
cross-cutting.md has: v1, v1.1

v1.1 adds: "Caching: Redis cache-aside on read-heavy endpoints"

This track has HTTP endpoints → constraint applies.
Phase 1 (completed): had GET endpoints → would need caching. Log TRACK_EXTENSION.
Phase 2 (starting now): will have more GET endpoints → apply caching from the start.

Update metadata.json: cc_version_current = "v1.1"
```
