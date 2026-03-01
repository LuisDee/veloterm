# Architect Hooks

These hooks run during Conductor's `/conductor:implement` at specific points. They are instructions for the implementing agent (human or AI), NOT automated event handlers.

**Activation:** These hooks are active when `conductor/workflow.md` contains the marker:
```
<!-- ARCHITECT:HOOKS — Read architect/hooks/*.md for additional workflow steps -->
```

If the marker is removed, hooks are disabled — graceful degradation.

## Hook Schedule

| # | Hook | When | Purpose |
|---|------|------|---------|
| 01 | 01-constraint-update-check.md | Before starting ANY new phase | Detect mid-track cross-cutting version changes and adopt new constraints for remaining phases |
| 02 | 02-interface-verification.md | Before implementing code that consumes another track's API or events | Verify the contract matches reality and decide: trust, verify, or mock |
| 03 | 03-discovery-check.md | After completing each task | Identify emergent work: new tracks, extensions, dependencies, cross-cutting changes |
| 04 | 04-phase-validation.md | Before marking a phase complete | Verify cross-cutting compliance for all work done in this phase |
| 05 | 05-wave-sync.md | After marking a track complete | Process discoveries, run quality gate, advance to next wave |

## Quick Reference

**Every phase boundary:** Read Hook 01 (constraint check) + Hook 04 (phase validation)
**Every task completion:** Read Hook 03 (discovery check)
**Before using another track's API:** Read Hook 02 (interface verification)
**After track completion:** Read Hook 05 (wave sync)

## Context Header Lifecycle

1. Architect generates `brief.md` with a context header (constraints, interfaces, dependencies)
2. Conductor generates `spec.md` at implementation time, preserving the context header at top
3. During implementation, hooks read constraints from `architect/cross-cutting.md` directly (not from the header — the header is for initial orientation)
