# Official Rust Style Guide Summary

This document summarizes the official Rust style guide from the Rust repository. All code must be formatted with `rustfmt` and pass `clippy` without warnings.

_Source: [Rust Style Guide](https://github.com/rust-lang/rust/tree/main/src/doc/style-guide/src)_

## 1. Guiding Principles

When style decisions conflict, prioritize in this order:

1. **Readability** — Scan-ability, avoiding misleading formatting, accessibility, legibility in plain-text contexts (diffs, error messages).
2. **Aesthetics** — Visual appeal, alignment with broader language conventions.
3. **Technical specifics** — VCS compatibility (preserving diffs, merge-friendliness), preventing horizontal indentation creep, efficient use of vertical space.
4. **Application** — Practicality for both manual formatting and automated tools (`rustfmt`), internal consistency.

## 2. Formatting Basics

- **Indentation:** 4 spaces per level. Never use tabs.
- **Maximum line width:** 100 characters.
- **Trailing commas:** Use trailing commas in comma-separated lists followed by newlines.
- **Blank lines:** Separate items with zero or one blank line. Never use two or more consecutive blank lines.
- **No trailing whitespace.**
- **Prefer block indentation** over visual indentation for smaller diffs.

## 3. Comments

- **Prefer line comments** (`//`) over block comments (`/* */`).
- Comments should be complete sentences with proper capitalization and punctuation.
- Comment-only lines should be limited to 80 characters or the max line width, whichever is smaller.
- **Doc comments:** Use `///` for item documentation. Reserve `//!` for module/crate-level docs.
- **Attributes:** Place each attribute on its own line. Consolidate multiple `derive` attributes into a single `#[derive(...)]`.

## 4. Naming Conventions

| Kind | Convention | Example |
|------|-----------|---------|
| Types | `UpperCamelCase` | `PaneNode`, `GlyphAtlas` |
| Enum variants | `UpperCamelCase` | `Horizontal`, `Vertical` |
| Struct fields | `snake_case` | `pane_id`, `scroll_offset` |
| Functions / methods | `snake_case` | `render_frame`, `spawn_shell` |
| Local variables | `snake_case` | `cell_width`, `read_buf` |
| Macros | `snake_case` | `debug_log!` |
| Constants / statics | `SCREAMING_SNAKE_CASE` | `MAX_SCROLLBACK`, `DEFAULT_FONT_SIZE` |
| Modules | `snake_case` | `glyph_atlas`, `pane_tree` |

- For reserved keywords, use raw identifiers (`r#crate`) or append an underscore (`crate_`). Avoid phonetic misspellings.
- Minimize use of `#[path]` attributes in module declarations.

## 5. Items (Module-Level)

### Ordering

1. `extern crate` statements (alphabetical)
2. `use` statements (version-sorted, `self` and `super` before other names)
3. Module declarations
4. Items

### Functions

- Standard signature order: `[pub] [unsafe] [extern ["ABI"]] fn name(args) -> return_type`
- When signatures exceed line width, break after the opening parenthesis with each argument on its own indented line with trailing comma.

### Structs / Enums

- Opening brace on the same line as the declaration.
- Each field/variant on its own indented line with trailing comma.
- Small struct variants in enums may stay single-line without trailing comma.

### Generics and Where Clauses

- Prefer single-line generics clauses. No spaces before/after `<` or before `>`. Spaces after commas.
- Multi-line generics: each parameter on its own indented line with trailing comma.
- `where` clauses on new lines, each bound indented, trailing commas.

### Imports

- Format on one line when possible, no spaces around braces.
- Within import groups: version-sort with `self`/`super` first, globs last.
- Nested imports: multi-line with each on a separate indented line.

## 6. Statements

### Let Statements

- Spaces after colons and around equals signs. No space before semicolons.
- Single-line preferred. If breaking needed, break after `=` with block indentation.
- For `let-else`: never break between `else` and `{`. Always break before `}`.

### Macros in Statement Position

- Use parentheses or square brackets as delimiters, terminate with semicolon.
- No spaces around name, `!`, delimiters, or `;`.

### Expressions in Statement Position

- No space between expression and semicolon.
- Terminate with semicolons unless ending with a block or representing a block's value.

## 7. Expressions

### Blocks

- Newlines after `{` and before `}` unless single-line rules apply.
- Keywords (`unsafe`, `async`) on same line as opening brace with single space.
- Empty blocks: `{}`.
- Single-line blocks: allowed when in expression position, containing a single-line expression with no statements/comments, with spaces inside braces.

### Closures

- No extra spaces before first `|` (unless prefixed by `move`).
- Space between `||` and expression.
- Omit braces when possible.

### Control Flow

- No extraneous parentheses for `if`/`while` conditions (unless clarifying complex logic).
- For broken control lines: opening brace on a new line, unindented.

### Function / Method Calls

- No space between function name and `(`.
- Single-line: no trailing comma.
- Multi-line: each argument indented, trailing comma after last argument.
- Method calls: no spaces around `.`.

### Chains

- Single line if small.
- Multi-line: each element on own line, break before `.` and after `?`, block-indent subsequent lines.

### Match Expressions

- Break after `{` and before `}`.
- Block-indent arms once.
- No trailing comma if using block form.
- Never start pattern with `|`.
- Single expressions stay inline; multiple statements use block body.

## 8. Types

- **Arrays:** `[T]` no spaces; `[u32; 42]` space after semicolon.
- **Pointers:** `*const T`, `*mut T` — no space after asterisk.
- **References:** `&'a T`, `&mut T` — no space after ampersand.
- **Generics:** `Foo::Bar<T, U, V>` — spaces after commas only.
- **Tuples:** `(A, B, C, D)` — spaces after commas, no trailing comma.
- **Trait objects:** `T + T + T` — single spaces between types and `+`.
- **Paths:** No spaces around `::` or angle brackets.
- When types span multiple lines, break at outermost scope first. For trait bounds with `+`, break before every `+` and block-indent.

## 9. Cargo.toml Conventions

- Blank lines between sections, not within them.
- Sort keys alphabetically within sections (except `[package]`).
- `[package]` section goes first: `name` and `version` at top, `description` last.
- Use bare (unquoted) keys for standard names.
- Single spaces around `=`.
- Arrays: single line when feasible; multi-line with block indentation and trailing commas.
- Short tables inline with `{}`; longer tables as separate `[section]`.

## 10. Expression-Oriented Style

Leverage Rust's expression-oriented design:

```rust
// Preferred
let x = if y { 1 } else { 0 };

// Avoid
let x;
if y {
    x = 1;
} else {
    x = 0;
}
```

## 11. Tooling Enforcement

- **`rustfmt`**: All code must be formatted with `rustfmt`. Run `cargo fmt` before every commit.
- **`clippy`**: All code must pass `cargo clippy` with no warnings. Use `#[allow(clippy::...)]` only with a justifying comment.
- **Configuration**: Use a `rustfmt.toml` at the project root if any defaults need overriding (e.g., `max_width = 100`).

**BE CONSISTENT.** When editing code, match the existing style.
