# Commit & Push Hygiene Rules

## MANDATORY: Commit Early, Commit Often

These rules exist because uncommitted work was catastrophically lost on 2026-02-21.
Modifications to ~20 tracked files (conductor dashboard UI, scrolling fixes, escape
query fixes, highlighting improvements) were wiped from the working tree with no
recovery path. Only untracked files (new directories) survived.

### Rules for Claude Code Sessions

1. **Commit after every logical unit of work** — a single feature, fix, or refactor.
   Do NOT accumulate changes across multiple files without committing.

2. **Push after every commit** — local commits are not safe. Push to remote immediately.

3. **Before making edits**, run `git status` to understand what's uncommitted. If there
   are significant uncommitted changes, commit them FIRST before starting new work.

4. **After running tests successfully**, commit immediately. Don't continue to the next
   task without committing passing state.

5. **Never leave a session with uncommitted work**. If interrupted, commit a WIP.

6. **Prefer multiple small commits** over one large commit. Each should compile and
   ideally pass tests, but a WIP commit is better than lost work.

### Commit Message Format

```
type(scope): short description

- bullet points for multi-change commits
```

Types: feat, fix, chore, docs, refactor, test, style

### Emergency Protocol

If you suspect working tree corruption:
1. `git stash` immediately — captures all tracked modifications
2. `git status` — check what survived
3. Commit any untracked files (`git add` + `git commit`)
4. Push everything

### What Git Protects (and What It Doesn't)

- **Tracked + committed**: Safe. Recoverable from reflog even after reset.
- **Tracked + staged**: Recoverable via `git fsck --lost-found` (dangling blobs).
- **Tracked + unstaged modifications**: VULNERABLE. A `git checkout .` or `git restore .` destroys them with no recovery.
- **Untracked files**: Survive `git checkout .` and `git restore .`, but destroyed by `git clean -f`.
