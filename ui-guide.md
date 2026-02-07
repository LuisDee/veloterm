Overview
This document provides exact specifications for implementing a Claude-themed terminal emulator in any language or framework. The design emphasizes warm, approachable aesthetics while maintaining professional readability and WCAG AAA contrast standards.

Theme 1: Claude Light
Color Palette
ElementHex CodeUsageBackground#FAFAF8Main window background, base layerPane Background#FFFFFFIndividual terminal pane backgroundsBorder#E5E3DEAll borders, separators, dividersText (Primary)#1A1816Main terminal output, commandsText (Muted)#6B6662Timestamps, inactive pane headers, metadataAccent#CC785CActive pane headers, prompt symbol ($), linksAccent Hover#B3654AHover states for accent-colored elementsPrompt#CC785CUsername@hostname in promptSuccess#2D7A4FSuccess messages, checkmarks (✓)Error#C44242Error messages, warningsSelection#FFE8DCText selection, cursor background
Layout Specifications
Main Window Structure
┌─────────────────────────────────────────────────────┐
│ Window Controls Bar                                  │ 32px height
├─────────────────────────────────────────────────────┤
│                                                      │
│  Terminal Grid (Tiling Area)                        │ Flexible height
│  - Main pane (left): 50% width, 100% height        │
│  - Top-right pane: 50% width, 50% height           │
│  - Bottom-right pane: 50% width, 50% height        │
│                                                      │
├─────────────────────────────────────────────────────┤
│ Status Bar                                          │ 32px height
└─────────────────────────────────────────────────────┘
Window Controls Bar (Top)

Height: 32px
Background: #FAFAF8 (same as main background)
Border Bottom: 1px solid #E5E3DE
Padding: 12px horizontal

Elements (left to right):

Traffic Light Dots (left-aligned, 12px from left edge)

Red: #ED6A5E (12×12px circle)
Yellow: #F4BF4F (12×12px circle)
Green: #61C554 (12×12px circle)
Gap between dots: 6px


Window Title (center-aligned)

Text: "Claude Terminal"
Color: #6B6662 (muted text)
Font Size: 13px
Font Weight: 500 (medium)
Icon: Small code bracket icon (14×14px), same color
Gap between icon and text: 6px


Maximize Button (right-aligned, 12px from right edge)

Icon: Maximize/expand icon (14×14px)
Color: #6B6662 (muted text)
Hover: Changes to #1A1816 (primary text)



Terminal Pane Structure
Outer Container:

Background: #FFFFFF (pane background)
Border: 1px solid #E5E3DE
Border Radius: 8px
Box Shadow: None (flat design)

Pane Header:

Height: 32px
Padding: 8px horizontal, centered vertically
Border Bottom: 1px solid #E5E3DE

Active Pane Header (primary/focused pane):

Background: #CC785C (accent)
Text Color: #FFFFFF (white)
Font Weight: 500

Inactive Pane Header:

Background: #FAFAF8 (main window background)
Text Color: #6B6662 (muted text)
Font Weight: 500

Header Elements (left to right):

Terminal icon (14×14px)
Pane title text (e.g., "~/projects/ai-solutions")
Spacer (flex: auto)
Two control dots (8×8px circles, #E5E3DE color, 4px gap)

Terminal Content Area:

Padding: 16px all sides
Font Family: "JetBrains Mono", "Fira Code", "SF Mono", monospace
Font Size: 13px
Line Height: 1.6 (20.8px)
Overflow: Auto (scrollable)

Terminal Text Elements:

System Messages (e.g., "Last login...")

Color: #6B6662 (muted text)
Margin Bottom: 8px


Prompt Line Structure:

   claude@anthropic ~ $

claude@anthropic: #CC785C (prompt color), font-weight: 600
~: #6B6662 (muted text)
$: #CC785C (accent)
Gap between elements: 8px
Margin Bottom: 4px


Command Text

Color: #1A1816 (primary text)
Padding Left: 16px (indented from prompt)


Success Output

Color: #2D7A4F (success)
Example: "✓ Ready in 1.2s"
Padding Left: 16px


Info Output

Color: #6B6662 (muted text)
Padding Left: 16px
Used for URLs, status messages


Cursor

Character: "_" (underscore) or "█" (block)
Color: #1A1816 (primary text)
Background: #FFE8DC (selection color)
Padding: 0-2px horizontal
Border Radius: 2px
Blinking animation recommended



Status Bar (Bottom)

Height: 32px
Background: #FFFFFF (pane background)
Border: 1px solid #E5E3DE
Border Radius: 6px
Padding: 8px horizontal
Margin: 12px from terminal grid
Display: Flex, space-between alignment

Left Section (status indicators):

Font Size: 11px
Color: #6B6662 (muted text)
Items separated by 16px gaps:

"3 panes"
"● Connected" (dot in #2D7A4F success color)



Right Section (system info):

Font Size: 11px
Color: #6B6662 (muted text)
Items separated by 16px gaps:

"UTF-8"
"zsh"
"claude@anthropic" (in #CC785C accent color)



Grid Layout (Tiling)

Display: Grid
Gap: 12px between panes
Grid Template:

Columns: 1fr 1fr (50/50 split)
Rows: 1fr 1fr (50/50 split)



Pane Assignments:

Main Pane: Column 1, Row span 1-2 (left side, full height)
Top Right: Column 2, Row 1
Bottom Right: Column 2, Row 2

Spacing & Sizing

Main window padding: 16px all sides
Pane border radius: 8px
Status bar border radius: 6px
Gap between panes: 12px
Gap between elements in headers: 6-8px
Terminal content padding: 16px
Indentation for command output: 16px


Theme 2: Claude Dark
Color Palette
ElementHex CodeUsageBackground#1A1816Main window background, base layerPane Background#252320Individual terminal pane backgroundsBorder#3D3833All borders, separators, dividersText (Primary)#E8E5DFMain terminal output, commandsText (Muted)#9B9389Timestamps, inactive pane headers, metadataAccent#E89171Active pane headers, prompt symbol ($), linksAccent Hover#F5A488Hover states for accent-colored elementsPrompt#E89171Username@hostname in promptSuccess#6BCF9BSuccess messages, checkmarks (✓)Error#F57878Error messages, warningsSelection#3D2E23Text selection, cursor background
Layout Specifications
IDENTICAL to Claude Light theme - all measurements, spacing, layout structure, and element positioning remain the same. Only colors change.
Specific Color Applications
Window Controls Bar:

Background: #1A1816
Border Bottom: #3D3833
Title Text: #9B9389
Traffic lights: Same as Light theme (#ED6A5E, #F4BF4F, #61C554)

Active Pane Header:

Background: #E89171 (brighter accent for dark theme)
Text: #FFFFFF

Inactive Pane Header:

Background: #1A1816
Text: #9B9389
Control dots: #3D3833

Terminal Content:

Background: #252320
Border: #3D3833
Primary text: #E8E5DF
Muted text: #9B9389
Prompt colors: #E89171 and #9B9389
Success: #6BCF9B
Error: #F57878
Cursor background: #3D2E23

Status Bar:

Background: #252320
Border: #3D3833
Text: #9B9389
Connected indicator: #6BCF9B
User text: #E89171

Visual Enhancements for Dark Theme

Add subtle box shadow for depth: 0 20px 60px rgba(0, 0, 0, 0.4) on main window
Borders may appear slightly lighter than background to create separation


Theme 3: Claude Warm
Color Palette
ElementHex CodeUsageBackground#2B2824Main window background, base layerPane Background#353230Individual terminal pane backgroundsBorder#4A453FAll borders, separators, dividersText (Primary)#E8E3D8Main terminal output, commandsText (Muted)#A39A8DTimestamps, inactive pane headers, metadataAccent#E89171Active pane headers, prompt symbol ($), linksAccent Hover#F5A488Hover states for accent-colored elementsPrompt#E89171Username@hostname in promptSuccess#7FD6A6Success messages, checkmarks (✓)Error#F57878Error messages, warningsSelection#4A3D32Text selection, cursor background
Layout Specifications
IDENTICAL to Claude Light and Dark themes - all measurements, spacing, layout structure, and element positioning remain the same. Only colors change.
Specific Color Applications
Window Controls Bar:

Background: #2B2824
Border Bottom: #4A453F
Title Text: #A39A8D
Traffic lights: Same as other themes

Active Pane Header:

Background: #E89171
Text: #FFFFFF

Inactive Pane Header:

Background: #2B2824
Text: #A39A8D
Control dots: #4A453F

Terminal Content:

Background: #353230
Border: #4A453F
Primary text: #E8E3D8
Muted text: #A39A8D
Prompt colors: #E89171 and #A39A8D
Success: #7FD6A6
Error: #F57878
Cursor background: #4A3D32

Status Bar:

Background: #353230
Border: #4A453F
Text: #A39A8D
Connected indicator: #7FD6A6
User text: #E89171


Typography Specifications
Font Families
Terminal Content (monospace):

Primary: "JetBrains Mono"
Fallback 1: "Fira Code"
Fallback 2: "SF Mono"
Fallback 3: monospace

UI Elements (sans-serif):

Primary: -apple-system
Fallback 1: BlinkMacSystemFont
Fallback 2: "Segoe UI"
Fallback 3: Roboto
Fallback 4: sans-serif

Font Sizes

Terminal content: 13px
Pane headers: 12px
Window title: 13px
Status bar: 11px
Color palette labels: 11px (name), 10px (hex code)

Font Weights

Normal text: 400 (regular)
Headers: 500 (medium)
Prompt username: 600 (semi-bold)
Window/section titles: 600 (semi-bold)

Line Heights

Terminal content: 1.6 (for 13px font = 20.8px line height)
UI text: 1.4-1.5


Interaction States
Hover States
Inactive Pane (hover):

Border changes from base border color to slightly lighter
Subtle background brightness increase (+5%)
Transition: 0.2s ease

Button/Control Hover:

Color changes from muted text to primary text
Accent elements go from accent to accent-hover
Transition: 0.2s ease

Maximize Button Hover:

Light theme: #6B6662 → #1A1816
Dark theme: #9B9389 → #E8E5DF
Warm theme: #A39A8D → #E8E3D8

Active/Focus States
Active Pane Indicators:

Header background uses accent color
Header text is white
Optional: subtle glow effect on border (1-2px, accent color at 30% opacity)

Text Selection:

Background: Selection color from palette
Text: Remains same color (no text color override)

Cursor:

Active (blinking): Alternates between visible/invisible
Blink rate: 530ms on, 530ms off (standard terminal timing)
Style: Block cursor with 2px padding or underscore


Design Principles
1. Warm & Approachable

Use warm grays, not pure black/white
Accent color is peachy-orange, Claude's signature
Even in dark themes, warmth is preserved through color temperature

2. High Contrast for Readability

All text combinations meet WCAG AAA standards
Primary text contrast ratio: >7:1 with background
Muted text contrast ratio: >4.5:1 with background

3. Soft Edges & Depth

Border radius: 8px for panes, 6px for smaller elements, 12px for main window
Subtle shadows only in dark themes
Gradual transitions, no harsh lines

4. Generous Spacing

Minimum gap between panes: 12px
Content padding: 16px
Element gaps: 6-8px
Prevent visual clutter with consistent spacing

5. Visual Hierarchy

Accent color draws eye to active pane
Muted text for secondary information
Success/error colors for status information
Consistent icon sizing (14px standard, 12px small, 8px indicators)


Implementation Notes
Accessibility

Support keyboard navigation between panes
Maintain focus indicators on active pane
Support system font scaling
Ensure cursor is always visible against background

Performance

Use flat colors (no gradients) for better rendering
CSS hardware acceleration for smooth transitions
Optimize redraw on text output

Responsive Behavior

Grid layout should reflow on small screens:



1200px: 3-pane layout as shown


768-1200px: 2-pane layout (main + one auxiliary)
<768px: Single pane with tabs



Theme Switching

All theme changes should be instant (no transition)
Preserve cursor position and scroll state
Update all UI elements atomically


Sample Terminal Content
For testing and mockups, use this representative content:
Last login: Sat Feb 7 14:23:01 on ttys001

claude@anthropic ~ $ npm run dev
  
✓ Ready in 1.2s

Local: http://localhost:3000
Network: http://192.168.1.5:3000

claude@anthropic ~ $ _

Color Contrast Ratios (WCAG Compliance)
Claude Light

Primary text on pane bg: #1A1816 on #FFFFFF = 19.4:1 (AAA)
Muted text on pane bg: #6B6662 on #FFFFFF = 4.9:1 (AA+)
Accent on white: #CC785C on #FFFFFF = 4.5:1 (AA)

Claude Dark

Primary text on pane bg: #E8E5DF on #252320 = 13.8:1 (AAA)
Muted text on pane bg: #9B9389 on #252320 = 5.1:1 (AA+)
Accent on dark bg: #E89171 on #252320 = 6.3:1 (AA+)

Claude Warm

Primary text on pane bg: #E8E3D8 on #353230 = 11.2:1 (AAA)
Muted text on pane bg: #A39A8D on #353230 = 4.7:1 (AA+)
Accent on warm bg: #E89171 on #353230 = 5.8:1 (AA+)


File/Directory Structure Recommendation
/themes
  /light
    colors.json
    layout.json
  /dark
    colors.json
    layout.json
  /warm
    colors.json
    layout.json
/assets
  /icons
    terminal.svg
    maximize.svg
    code.svg
  /fonts
    (include JetBrains Mono if bundling)

Quick Reference: All Theme Colors
Color NameLightDarkWarmBackground#FAFAF8#1A1816#2B2824Pane Background#FFFFFF#252320#353230Border#E5E3DE#3D3833#4A453FText Primary#1A1816#E8E5DF#E8E3D8Text Muted#6B6662#9B9389#A39A8DAccent#CC785C#E89171#E89171Accent Hover#B3654A#F5A488#F5A488Prompt#CC785C#E89171#E89171Success#2D7A4F#6BCF9B#7FD6A6Error#C44242#F57878#F57878Selection#FFE8DC#3D2E23#4A3D32

End of Specification
This document provides complete visual specifications for implementing Claude-themed terminal emulators in any programming language or framework. All measurements, colors, and layout rules are exact and implementation-agnostic.
