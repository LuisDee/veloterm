# VeloTerm Requirements — MCP Fix, File Browser Overhaul, Git Review Overhaul

## User Requirements (Verbatim)

### 1. Fix VeloTerm MCP Launch
The VeloTerm MCP server's `veloterm_launch` fails every attempt. It builds the bare binary (not the .app bundle), which means:
- Scale factor is 1.0 instead of 2.0 on Retina
- The GetWindowID-based window detection timed out after 30 seconds despite the app actually running (logs confirmed PTY was spawned)
- The window likely wasn't discoverable under the expected name because it wasn't launched via `open` as an .app bundle

Once fixed this will be a powerful tool to leverage for visual testing.

### 2. File Browser Overhaul
The file browser and git panels are hard coded to root — they do not look at the CWD of the current active pane. This is what they should do. The file browser looks bad and doesn't function properly. Requirements:

- **Target working directory should be the current CWD of the active pane**
- **Click to expand folders** — thorough high-quality tests for this
- **Icons for different file types** — visual differentiation
- **Double click files to view them in the right pane**
- **Text in viewer should be highlightable and copyable**
- **Research what IDE file explorers have (e.g., VS Code)** and create a comprehensive high-quality TDD test suite for all this functionality
- The conductor track dashboard should also respect CWD if it doesn't already
- Every plan must be reviewed, every implementation must be reviewed
- After implementation: manually test, judge whether it's been implemented to its fullest potential, then recreate a plan to add fixes and adjustments and iterate

### 3. Git Review / Changelog Overhaul
The git changelog is also buggy — same procedure must be applied:

- **Currently checks at root which isn't a git repo** — must detect CWD of active pane and find the git repo
- **Navigate to any git repo and validate it functions**
- **List of changed files split into: staged, unstaged, and untracked**
- **Click each file to show the diff in the pane to the right**
- Same review and iteration cycle as file browser

### 4. Process Requirements
- Fix MCP first — once working, use it as the primary visual testing tool
- Use agent teams (6+ agents) to distribute tasks
- Every plan must be reviewed by a reviewer agent
- Every implementation must be reviewed by a reviewer agent
- Harness sequential thinking, Context7, and web search for research
- TDD approach: comprehensive test suites first, then implement until green
- After implementation: visual review via MCP screenshots, iterate with fixes

## Implementation Order
1. **Phase 0**: Fix VeloTerm MCP launch (prerequisite for all visual testing)
2. **Phase 1**: File Browser — CWD detection, navigation, icons, viewer, tests
3. **Phase 2**: Git Review — CWD detection, staged/unstaged/untracked, diff viewer, tests
4. **Phase 3**: Visual review and iteration cycle using MCP screenshots
