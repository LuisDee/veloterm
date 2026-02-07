# Product Guidelines — VeloTerm

## Tone & Voice

VeloTerm communicates with a **technical and precise** voice. All documentation, UI copy, error messages, and project communications assume the reader is a competent software developer.

- **Be direct.** Say what you mean in the fewest words necessary. No filler, no marketing fluff.
- **Be specific.** Prefer concrete numbers and measurable claims over vague qualifiers. "Input latency under 10ms" not "blazing fast input."
- **Be honest.** If there's a limitation, state it. If a feature isn't implemented yet, say so. Don't hedge with weasel words.
- **Assume competence.** Don't explain what a PTY is. Don't explain what Ctrl+C does. Write for developers who live in the terminal.

## Brand Personality

VeloTerm is a **speed-obsessed craftsman**. Every architectural decision, dependency choice, and feature design is justified by performance data and engineering rigor.

- Lead with evidence: benchmarks, profiling data, architectural rationale.
- "We measured it, here's the proof" over "trust us, it's fast."
- Respect the user's time — in the product, in the docs, in every interaction.
- The project earns credibility through engineering quality, not marketing.

## Visual Design Philosophy

VeloTerm's UI chrome follows a **clean and functional** design philosophy.

- **Terminal content is primary.** UI chrome exists to support the terminal, not compete with it.
- **Visible but understated.** Dividers (2-4px), tab bar, and focus indicators should be clearly visible without being distracting.
- **Visual affordance matters.** New users should be able to discover split panes, resize handles, and tabs by looking at the interface. Don't hide functionality behind invisible interactions.
- **No decorative elements.** Every pixel of UI chrome serves a functional purpose.
- **Consistent visual language.** Borders, highlights, and interactive elements use a coherent set of colors derived from the active theme.

## Terminology

VeloTerm uses **standard terminal terminology** throughout the codebase, documentation, configuration, and UI.

- Use established terms: "pane", "tab", "scrollback", "shell", "cursor", "PTY", "escape sequence."
- Do not invent new names for existing concepts. Users should feel immediately oriented.
- When referencing keyboard shortcuts in documentation, always specify both platforms: "Cmd+Shift+D (macOS) / Ctrl+Shift+D (Linux)."

## Error & Feedback Philosophy

VeloTerm is **verbose and transparent by default** with configurable log levels.

- Surface all warnings and errors visibly on startup and during operation. Developers want to know what's happening under the hood.
- Config parse failures, GPU initialization issues, font loading problems, and PTY errors are reported inline with actionable context.
- Follow best-practice log levels: `error`, `warn`, `info`, `debug`, `trace`.
- Default log level: `info`. Users can adjust via configuration (e.g., `log_level: warn` to reduce noise).
- Error messages must include: what went wrong, why it matters, and what the user can do about it.

## Documentation Standards

- All public APIs, configuration options, and CLI flags must be documented.
- Documentation lives as close to the code as possible (rustdoc comments for Rust code).
- README and user-facing docs are concise and scannable — use tables, bullet points, and code blocks.
- Changelogs follow Keep a Changelog format. Every user-facing change gets an entry.

## Quality Standards

- Every performance claim must be backed by reproducible benchmarks.
- Configuration defaults must be opinionated and well-chosen — the terminal should work excellently out of the box.
- Keyboard shortcuts must not conflict with common shell or TUI application bindings.
- Cross-platform behavior must be consistent — same features, same keybinding logic, same rendering quality on both macOS and Linux.
