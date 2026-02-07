# UI Guide — VeloTerm (Claude-Themed)

This document provides exact specifications for VeloTerm's visual design. The design emphasizes warm, approachable aesthetics while maintaining professional readability and WCAG AAA contrast standards.

## Design Principles

1. **Warm & Approachable** — Use warm grays, not pure black/white. Accent color is peachy-orange (Claude's signature). Even in dark themes, warmth is preserved through color temperature.
2. **High Contrast for Readability** — All text combinations meet WCAG AAA standards. Primary text contrast ratio: >7:1. Muted text contrast ratio: >4.5:1.
3. **Soft Edges & Depth** — Border radius: 8px for panes, 6px for smaller elements, 12px for main window. Subtle shadows only in dark themes. Gradual transitions, no harsh lines.
4. **Generous Spacing** — Minimum gap between panes: 12px. Content padding: 16px. Element gaps: 6-8px. Prevent visual clutter with consistent spacing.
5. **Visual Hierarchy** — Accent color draws eye to active pane. Muted text for secondary information. Success/error colors for status. Consistent icon sizing (14px standard, 12px small, 8px indicators).

---

## Themes

VeloTerm ships with three built-in Claude themes. All themes share identical layout, spacing, and structure — only colors differ.

### Theme 1: Claude Light

| Element | Hex Code | Usage |
|---------|----------|-------|
| Background | `#FAFAF8` | Main window background, base layer |
| Pane Background | `#FFFFFF` | Individual terminal pane backgrounds |
| Border | `#E5E3DE` | All borders, separators, dividers |
| Text (Primary) | `#1A1816` | Main terminal output, commands |
| Text (Muted) | `#6B6662` | Timestamps, inactive pane headers, metadata |
| Accent | `#CC785C` | Active pane headers, prompt symbol ($), links |
| Accent Hover | `#B3654A` | Hover states for accent-colored elements |
| Prompt | `#CC785C` | Username@hostname in prompt |
| Success | `#2D7A4F` | Success messages, checkmarks |
| Error | `#C44242` | Error messages, warnings |
| Selection | `#FFE8DC` | Text selection, cursor background |

**WCAG Contrast Ratios:**
- Primary text on pane bg: `#1A1816` on `#FFFFFF` = 19.4:1 (AAA)
- Muted text on pane bg: `#6B6662` on `#FFFFFF` = 4.9:1 (AA+)
- Accent on white: `#CC785C` on `#FFFFFF` = 4.5:1 (AA)

### Theme 2: Claude Dark

| Element | Hex Code | Usage |
|---------|----------|-------|
| Background | `#1A1816` | Main window background, base layer |
| Pane Background | `#252320` | Individual terminal pane backgrounds |
| Border | `#3D3833` | All borders, separators, dividers |
| Text (Primary) | `#E8E5DF` | Main terminal output, commands |
| Text (Muted) | `#9B9389` | Timestamps, inactive pane headers, metadata |
| Accent | `#E89171` | Active pane headers, prompt symbol ($), links |
| Accent Hover | `#F5A488` | Hover states for accent-colored elements |
| Prompt | `#E89171` | Username@hostname in prompt |
| Success | `#6BCF9B` | Success messages, checkmarks |
| Error | `#F57878` | Error messages, warnings |
| Selection | `#3D2E23` | Text selection, cursor background |

**WCAG Contrast Ratios:**
- Primary text on pane bg: `#E8E5DF` on `#252320` = 13.8:1 (AAA)
- Muted text on pane bg: `#9B9389` on `#252320` = 5.1:1 (AA+)
- Accent on dark bg: `#E89171` on `#252320` = 6.3:1 (AA+)

**Visual enhancements:** Subtle box shadow for depth: `0 20px 60px rgba(0, 0, 0, 0.4)` on main window.

### Theme 3: Claude Warm

| Element | Hex Code | Usage |
|---------|----------|-------|
| Background | `#2B2824` | Main window background, base layer |
| Pane Background | `#353230` | Individual terminal pane backgrounds |
| Border | `#4A453F` | All borders, separators, dividers |
| Text (Primary) | `#E8E3D8` | Main terminal output, commands |
| Text (Muted) | `#A39A8D` | Timestamps, inactive pane headers, metadata |
| Accent | `#E89171` | Active pane headers, prompt symbol ($), links |
| Accent Hover | `#F5A488` | Hover states for accent-colored elements |
| Prompt | `#E89171` | Username@hostname in prompt |
| Success | `#7FD6A6` | Success messages, checkmarks |
| Error | `#F57878` | Error messages, warnings |
| Selection | `#4A3D32` | Text selection, cursor background |

**WCAG Contrast Ratios:**
- Primary text on pane bg: `#E8E3D8` on `#353230` = 11.2:1 (AAA)
- Muted text on pane bg: `#A39A8D` on `#353230` = 4.7:1 (AA+)
- Accent on warm bg: `#E89171` on `#353230` = 5.8:1 (AA+)

### Quick Reference: All Theme Colors

| Color Name | Light | Dark | Warm |
|------------|-------|------|------|
| Background | `#FAFAF8` | `#1A1816` | `#2B2824` |
| Pane Background | `#FFFFFF` | `#252320` | `#353230` |
| Border | `#E5E3DE` | `#3D3833` | `#4A453F` |
| Text Primary | `#1A1816` | `#E8E5DF` | `#E8E3D8` |
| Text Muted | `#6B6662` | `#9B9389` | `#A39A8D` |
| Accent | `#CC785C` | `#E89171` | `#E89171` |
| Accent Hover | `#B3654A` | `#F5A488` | `#F5A488` |
| Prompt | `#CC785C` | `#E89171` | `#E89171` |
| Success | `#2D7A4F` | `#6BCF9B` | `#7FD6A6` |
| Error | `#C44242` | `#F57878` | `#F57878` |
| Selection | `#FFE8DC` | `#3D2E23` | `#4A3D32` |

---

## Layout Specifications

### Main Window Structure

```
┌─────────────────────────────────────────────────────┐
│ Window Controls Bar                                  │ 32px height
├─────────────────────────────────────────────────────┤
│                                                      │
│  Terminal Grid (Tiling Area)                         │ Flexible height
│  - Main pane (left): 50% width, 100% height         │
│  - Top-right pane: 50% width, 50% height            │
│  - Bottom-right pane: 50% width, 50% height         │
│                                                      │
├─────────────────────────────────────────────────────┤
│ Status Bar                                           │ 32px height
└─────────────────────────────────────────────────────┘
```

### Window Controls Bar (Top)

- Height: 32px
- Background: Same as main background
- Border Bottom: 1px solid border color
- Padding: 12px horizontal

**Elements (left to right):**

1. **Traffic Light Dots** (left-aligned, 12px from left edge)
   - Red: `#ED6A5E` (12x12px circle)
   - Yellow: `#F4BF4F` (12x12px circle)
   - Green: `#61C554` (12x12px circle)
   - Gap between dots: 6px

2. **Window Title** (center-aligned)
   - Text: "Claude Terminal"
   - Color: Muted text color
   - Font Size: 13px
   - Font Weight: 500 (medium)
   - Icon: Small code bracket icon (14x14px), same color
   - Gap between icon and text: 6px

3. **Maximize Button** (right-aligned, 12px from right edge)
   - Icon: Maximize/expand icon (14x14px)
   - Color: Muted text color
   - Hover: Changes to primary text color

### Terminal Pane Structure

**Outer Container:**
- Background: Pane background color
- Border: 1px solid border color
- Border Radius: 8px
- Box Shadow: None (flat design)

**Pane Header:**
- Height: 32px
- Padding: 8px horizontal, centered vertically
- Border Bottom: 1px solid border color

**Active Pane Header:**
- Background: Accent color
- Text Color: `#FFFFFF` (white)
- Font Weight: 500

**Inactive Pane Header:**
- Background: Main window background color
- Text Color: Muted text color
- Font Weight: 500

**Header Elements (left to right):**
1. Terminal icon (14x14px)
2. Pane title text (e.g., "~/projects/ai-solutions")
3. Spacer (flex: auto)
4. Two control dots (8x8px circles, border color, 4px gap)

**Terminal Content Area:**
- Padding: 16px all sides
- Font Family: "JetBrains Mono", "Fira Code", "SF Mono", monospace
- Font Size: 13px
- Line Height: 1.6 (20.8px)
- Overflow: Auto (scrollable)

### Terminal Text Elements

| Element | Color | Details |
|---------|-------|---------|
| System Messages | Muted text | e.g., "Last login...". Margin bottom: 8px |
| Prompt (`claude@anthropic`) | Prompt color | Font weight: 600 |
| Directory (`~`) | Muted text | — |
| Prompt symbol (`$`) | Accent color | — |
| Command Text | Primary text | Padding left: 16px |
| Success Output | Success color | e.g., "Ready in 1.2s". Padding left: 16px |
| Info Output | Muted text | URLs, status messages. Padding left: 16px |

Gap between prompt elements: 8px. Margin bottom for prompt: 4px.

### Cursor

- Style: Block (`█`) or underscore (`_`), configurable
- Color: Primary text color
- Background: Selection color
- Padding: 0-2px horizontal
- Border Radius: 2px
- Blink rate: 530ms on, 530ms off (standard terminal timing)

### Status Bar (Bottom)

- Height: 32px
- Background: Pane background color
- Border: 1px solid border color
- Border Radius: 6px
- Padding: 8px horizontal
- Margin: 12px from terminal grid
- Display: Flex, space-between alignment

**Left Section (status indicators):**
- Font Size: 11px
- Color: Muted text color
- Items: "3 panes", "Connected" (dot in success color)
- Gap between items: 16px

**Right Section (system info):**
- Font Size: 11px
- Color: Muted text color
- Items: "UTF-8", "zsh", "claude@anthropic" (in accent color)
- Gap between items: 16px

### Grid Layout (Tiling)

- Display: Grid
- Gap: 12px between panes
- Columns: 1fr 1fr (50/50 split)
- Rows: 1fr 1fr (50/50 split)

**Pane Assignments:**
- Main Pane: Column 1, Row span 1-2 (left side, full height)
- Top Right: Column 2, Row 1
- Bottom Right: Column 2, Row 2

---

## Typography Specifications

### Font Families

**Terminal Content (monospace):**
1. "JetBrains Mono" (primary)
2. "Fira Code" (fallback 1)
3. "SF Mono" (fallback 2)
4. monospace (fallback 3)

**UI Elements (sans-serif):**
1. -apple-system (primary)
2. BlinkMacSystemFont (fallback 1)
3. "Segoe UI" (fallback 2)
4. Roboto (fallback 3)
5. sans-serif (fallback 4)

### Font Sizes

| Element | Size |
|---------|------|
| Terminal content | 13px |
| Pane headers | 12px |
| Window title | 13px |
| Status bar | 11px |

### Font Weights

| Element | Weight |
|---------|--------|
| Normal text | 400 (regular) |
| Headers | 500 (medium) |
| Prompt username | 600 (semi-bold) |
| Window/section titles | 600 (semi-bold) |

### Line Heights

| Element | Line Height |
|---------|-------------|
| Terminal content | 1.6 (20.8px at 13px) |
| UI text | 1.4-1.5 |

---

## Interaction States

### Hover States

**Inactive Pane:**
- Border changes from base border color to slightly lighter
- Subtle background brightness increase (+5%)
- Transition: 0.2s ease

**Button/Control:**
- Color changes from muted text to primary text
- Accent elements go from accent to accent-hover
- Transition: 0.2s ease

**Maximize Button:**
- Light: `#6B6662` → `#1A1816`
- Dark: `#9B9389` → `#E8E5DF`
- Warm: `#A39A8D` → `#E8E3D8`

### Active/Focus States

**Active Pane:**
- Header background uses accent color
- Header text is white
- Optional: subtle glow effect on border (1-2px, accent color at 30% opacity)

**Text Selection:**
- Background: Selection color from palette
- Text: Remains same color (no text color override)

---

## Spacing & Sizing Summary

| Element | Value |
|---------|-------|
| Main window padding | 16px all sides |
| Pane border radius | 8px |
| Status bar border radius | 6px |
| Main window border radius | 12px |
| Gap between panes | 12px |
| Gap between header elements | 6-8px |
| Terminal content padding | 16px |
| Command output indentation | 16px |

---

## Responsive Behavior

- **≥1200px:** 3-pane layout as shown
- **768-1200px:** 2-pane layout (main + one auxiliary)
- **<768px:** Single pane with tabs

---

## Theme Switching

- All theme changes should be instant (no transition)
- Preserve cursor position and scroll state
- Update all UI elements atomically

---

## Accessibility

- Support keyboard navigation between panes
- Maintain focus indicators on active pane
- Support system font scaling
- Ensure cursor is always visible against background

---

## Implementation Notes

- Use flat colors (no gradients) for better GPU rendering performance
- Optimize redraw on text output
- Traffic light dot colors are consistent across all themes: Red `#ED6A5E`, Yellow `#F4BF4F`, Green `#61C554`

---

## Theme File Structure

```
assets/themes/
├── claude_light.toml
├── claude_dark.toml
└── claude_warm.toml
```
