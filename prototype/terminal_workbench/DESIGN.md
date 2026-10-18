---
name: Terminal Workbench
colors:
  surface: '#121221'
  surface-dim: '#121221'
  surface-bright: '#383849'
  surface-container-lowest: '#0d0d1c'
  surface-container-low: '#1a1a2a'
  surface-container: '#1e1e2e'
  surface-container-high: '#292839'
  surface-container-highest: '#333344'
  on-surface: '#e3e0f7'
  on-surface-variant: '#c3c6d2'
  inverse-surface: '#e3e0f7'
  inverse-on-surface: '#2f2f40'
  outline: '#8d919b'
  outline-variant: '#424750'
  surface-tint: '#a8c8ff'
  primary: '#b5cfff'
  on-primary: '#003062'
  primary-container: '#89b4fa'
  on-primary-container: '#094583'
  inverse-primary: '#305f9f'
  secondary: '#d9b9ff'
  on-secondary: '#3f1e66'
  secondary-container: '#593981'
  on-secondary-container: '#cda8fa'
  tertiary: '#88dbea'
  on-tertiary: '#00363d'
  tertiary-container: '#6cbfce'
  on-tertiary-container: '#004d56'
  error: '#ffb4ab'
  on-error: '#690005'
  error-container: '#93000a'
  on-error-container: '#ffdad6'
  primary-fixed: '#d5e3ff'
  primary-fixed-dim: '#a8c8ff'
  on-primary-fixed: '#001b3c'
  on-primary-fixed-variant: '#0e4685'
  secondary-fixed: '#eedbff'
  secondary-fixed-dim: '#d9b9ff'
  on-secondary-fixed: '#290350'
  on-secondary-fixed-variant: '#57377f'
  tertiary-fixed: '#9cefff'
  tertiary-fixed-dim: '#80d3e2'
  on-tertiary-fixed: '#001f24'
  on-tertiary-fixed-variant: '#004f58'
  background: '#121221'
  on-background: '#e3e0f7'
  surface-variant: '#333344'
typography:
  headline-lg:
    fontFamily: Geist
    fontSize: 1.25rem
    fontWeight: '600'
    lineHeight: 1.75rem
    letterSpacing: -0.015em
  headline-sm:
    fontFamily: Geist
    fontSize: 1rem
    fontWeight: '600'
    lineHeight: 1.375rem
    letterSpacing: -0.01em
  body-default:
    fontFamily: Geist
    fontSize: 0.8125rem
    fontWeight: '400'
    lineHeight: 1.1875rem
  body-bold:
    fontFamily: Geist
    fontSize: 0.8125rem
    fontWeight: '600'
    lineHeight: 1.1875rem
  body-compact:
    fontFamily: Geist
    fontSize: 0.75rem
    fontWeight: '400'
    lineHeight: 1rem
  code-default:
    fontFamily: JetBrains Mono
    fontSize: 0.8125rem
    fontWeight: '400'
    lineHeight: 1.25rem
  code-dense:
    fontFamily: JetBrains Mono
    fontSize: 0.75rem
    fontWeight: '400'
    lineHeight: 1.125rem
  label-default:
    fontFamily: Geist
    fontSize: 0.6875rem
    fontWeight: '500'
    lineHeight: 0.875rem
    letterSpacing: 0.02em
  label-mono:
    fontFamily: JetBrains Mono
    fontSize: 0.6875rem
    fontWeight: '500'
    lineHeight: 0.875rem
    letterSpacing: 0.01em
rounded:
  sm: 0.125rem
  DEFAULT: 0.25rem
  md: 0.375rem
  lg: 0.5rem
  xl: 0.75rem
  full: 9999px
spacing:
  gutter: 0.25rem
  gutter-desktop: 0.375rem
  margin: 0.5rem
  margin-desktop: 0.75rem
  space-xs: 0.125rem
  space-sm: 0.25rem
  space-md: 0.5rem
  space-lg: 0.75rem
  space-xl: 1rem
---

## Brand & Style

This design system targets systems engineers, cloud architects, DevOps practitioners, and terminal power users who demand precision, sustained visual ergonomics, and dense operational telemetry. It balances the utilitarian rigor of workstation clients like Xshell and VS Code with the refined, modern aesthetics of Termius.

The visual style blends **Technical Minimalism** with **Dense Functionalism**:
- High-density information architecture with zero non-functional decoration.
- Strict visual hierarchy driven by structural panels, crisp 1px separation lines, and disciplined syntax highlighting accents.
- Purpose-built for multi-monitor workstations, low-latency rendering, and long hours under low-light operating conditions.
- Deep, low-fatigue slate and charcoal canvas layers paired with vibrant terminal-grade status tokens.

## Colors

The palette relies on a multi-tiered dark foundation built from deep navy and charcoal values, offset by crisp structural borders and high-visibility status accents.

### Canvas & Surface Hierarchy
- **Base Canvas (Editor & Shell Terminal)**: `#11111b` — Absolute lowest elevation for maximum contrast with active code and terminal logs.
- **Surface Layer 1 (Active Panes & Center Workspaces)**: `#181824` — Primary work surface for interactive panes and splits.
- **Surface Layer 2 (Sidebars, Menus & Tool Shelves)**: `#1e1e2e` — Medium-elevation navigation trees, session managers, and headers.
- **Surface Layer 3 (Floating Overlays & Dialogs)**: `#252538` — Modals, context menus, and palette pickers.

### Borders & Dividers
- **Subtle Partition**: `#313244` — Standard container dividers, collapsed trees, and passive split-pane borders.
- **Active Focus & Hover Stroke**: `#45475a` — Hover states, input strokes, and inactive tab boundaries.
- **Accent Focus**: `#89b4fa` — Keyboard-focused inputs, active pane outlines, and active tab tops.

### Terminal & Status Accents
- **Terminal Green (Success / Connected)**: `#a6e3a1` — Active SSH tunnels, successful exit codes, transfer completions.
- **Vibrant Cyan (Info / Protocol)**: `#89dceb` — Host IPs, SFTP symlinks, protocol tags, and session indicators.
- **Alert Amber (Warning / High Load)**: `#f9e2af` — Disconnecting states, sudo confirmations, high CPU/RAM thresholds.
- **Error Red (Failure / Terminated)**: `#f38ba8` — Connection refused, broken pipes, critical error outputs.
- **Active Purple (Secondary Focus / Badges)**: `#cba6f7` — Keybinding chips, tag badges, and alternate command highlights.

## Typography

Typography enforces strict dual-role segregation: **Geist** governs application scaffolding, window navigation, and modal dialogues; **JetBrains Mono** powers terminal emulators, hex viewers, SSH session output, code views, and micro telemetry.

- **Legibility & Density**: UI body size is pinned to 13px (`0.8125rem`), enabling compact data tables, deep nested connection trees, and maximized terminal vertical real estate.
- **Tabular Figures**: Numeric outputs, bandwidth readouts, and latency indicators in JetBrains Mono use monospace figures to prevent layout jitter during live updates.
- **Font Rendering**: All type utilizes subpixel antialiasing (`-webkit-font-smoothing: antialiased`) to maintain stroke fidelity against `#11111b` and `#181824`.

## Layout & Spacing

The layout model is governed by a **Docked Pane & Split Grid** architecture calibrated for ultra-dense desktop environments. Rather than wide responsive fluid columns, the interface relies on tileable splitters, persistent tool rails, and compact horizontal panes.

### Layout Model
- **Primary Tool Rails**: Fixed 44px icon bar on extreme left or right.
- **Collapsible Drawers / Trees**: Default 240px width, resizable down to 180px or collapsable to 0.
- **Split Panes**: Resizable horizontal and vertical panes with 2px hit areas (`0.25rem` gutter padding around drag handles).
- **Workbench Workspaces**: Maximized viewport container without external canvas margins. Margins apply strictly to floating utility windows and connection detail drawers.

### Spacing Rules
- UI components adhere strictly to a 4px/8px incremental scale.
- Padding inside terminal viewports: `space-sm` (4px) horizontal and vertical to eliminate edge truncation while maximizing character columns.
- Tree and list item row heights are locked to 24px (compact) or 28px (standard).

## Elevation & Depth

This design system deliberately minimizes soft shadows, relying instead on **low-contrast outlines** and **tonal panel stratification** to preserve computational rendering speed and absolute clarity under complex split-screen setups.

### Structural Depth
- **Level 0 (Recessed Panels)**: `#11111b` — Shell canvas, file view tables. Defined by inward 1px solid borders (`#313244`).
- **Level 1 (Docked Frames)**: `#181824` — Active editor panes and workspace tiles.
- **Level 2 (Scaffolding)**: `#1e1e2e` — Global headers, status strips, and sidebar backgrounds.
- **Level 3 (Popovers & Palettes)**: `#252538` — Command palettes (`Ctrl+Shift+P`), autocomplete menus, and contextual dropdowns. Supported by a crisp 1px `#45475a` border and a tight ambient drop shadow: `0 8px 24px rgba(0, 0, 0, 0.6)`.

### Active Split & Focus Indication
Active terminal or editor splits receive an illuminated 1px border (`#89b4fa`) or a 2px top border on the corresponding tab header. Inactive panes are dimmed with an overlay of `rgba(17, 17, 27, 0.25)` and bordered with `#313244`.

## Shapes

The shape system adopts a **Soft (Level 1)** precision form factor. Curves are kept minimal to emulate industrial developer tools and prevent visual rounding distortion in multi-pane layouts.

- **Standard Elements (Buttons, Inputs, Badges, Tabs)**: 4px (`0.25rem`) corner radius.
- **Split Windows & Panes**: 0px radius at tile interfaces to provide clean, flush, non-overlapping seam lines.
- **Modals, Floating Panels & Tooltips**: 6px (`0.375rem`) to 8px (`0.5rem`) corner radius (`rounded-lg`) to clearly separate floating layers from the rigid docked tiles beneath them.

## Components

### Tabs (Terminal & Session)
- **Container**: Pinned to the top of each pane, height 32px, background `#11111b`, border-bottom 1px solid `#313244`.
- **Inactive Tab**: Background `#181824`, text `#9399b2`, 1px right border `#313244`. Hover turns text to `#cdd6f4`.
- **Active Tab**: Background `#1e1e2e`, text `#cdd6f4`, with a prominent 2px top accent line in `#89b4fa`. Includes an inline close glyph (`x`) that highlights to `#f38ba8` on hover.
- **Status Dot**: 6px circular indicator before the tab label (e.g., `#a6e3a1` for open SSH session, `#f9e2af` for background activity, `#f38ba8` for terminated session).

### Split Panes & Dividers
- **Sash Line**: 1px solid `#313244`.
- **Hover/Drag State**: Expands to an illuminated 2px `#89b4fa` guideline with a persistent `col-resize` or `row-resize` cursor.

### Buttons & Quick Actions
- **Primary**: Background `#89b4fa`, text `#11111b`, font-weight 600. Hover `#b4befe`. Active scale: none.
- **Secondary / Ghost**: Background `transparent`, border 1px solid `#45475a`, text `#cdd6f4`. Hover background `#313244`.
- **Dense Tool Button**: 24px x 24px icon button, 2px padding, radius 3px, text `#a6adc8`. Hover background `#313244` and text `#cdd6f4`.

### Inputs & Terminal Find Bar
- **Field**: Background `#11111b`, border 1px solid `#313244`, height 26px, padding `0 8px`, font JetBrains Mono 12px.
- **Focus**: Border `#89b4fa` with zero glow/box-shadow ring.
- **Toggles (Regex, Match Case)**: Inline 20px x 20px toggle buttons inside the search field. Selected state: background `#45475a`, text `#89b4fa`.

### Tree Views (Host & Directory Manager)
- **Row Height**: 22px fixed. Indent guides: 1px dotted `#313244`.
- **Hover**: Background `#252538`.
- **Selected**: Background `#313244`, text `#89b4fa`, with a 2px left border accent in `#89b4fa`.
- **Badges**: Protocol tags (e.g., `SSH`, `SFTP`, `TELNET`) set in JetBrains Mono 9px, uppercase, padding 1px 4px, background `#181824`, border 1px solid `#45475a`, color `#89dceb`.

### Progress Bars (File Transfer & Telemetry)
- **Track**: Height 4px (inline table) or 6px (panel transfer view), background `#313244`, border-radius 2px.
- **Fill**: Default `#89dceb` for active upload/download, transitioning to `#a6e3a1` upon verification/completion.
- **Stalled/Error Fill**: Solid `#f38ba8`.
- **Transfer Metadata**: Inline display of transfer rate (`MB/s`), elapsed time, and ETA rendered strictly in `label-mono` (`#a6adc8`).

### Command Palette & Keybinding Chips
- **Palette**: Centered floating overlay, width 560px, surface `#1e1e2e`, border 1px solid `#45475a`.
- **Keybinding Kbd**: Background `#11111b`, border 1px solid `#313244`, border-bottom 2px solid `#45475a`, border-radius 3px, font JetBrains Mono 10px, padding 1px 5px, color `#bac2de`.