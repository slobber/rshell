---
name: DevOps Precision Light
colors:
  surface: '#f8f9ff'
  surface-dim: '#cbdbf5'
  surface-bright: '#f8f9ff'
  surface-container-lowest: '#ffffff'
  surface-container-low: '#eff4ff'
  surface-container: '#e5eeff'
  surface-container-high: '#dce9ff'
  surface-container-highest: '#d3e4fe'
  on-surface: '#0b1c30'
  on-surface-variant: '#434655'
  inverse-surface: '#213145'
  inverse-on-surface: '#eaf1ff'
  outline: '#737686'
  outline-variant: '#c3c6d7'
  surface-tint: '#0053db'
  primary: '#004ac6'
  on-primary: '#ffffff'
  primary-container: '#2563eb'
  on-primary-container: '#eeefff'
  inverse-primary: '#b4c5ff'
  secondary: '#006c4a'
  on-secondary: '#ffffff'
  secondary-container: '#82f5c1'
  on-secondary-container: '#00714e'
  tertiary: '#4d556b'
  on-tertiary: '#ffffff'
  tertiary-container: '#656d84'
  on-tertiary-container: '#eef0ff'
  error: '#ba1a1a'
  on-error: '#ffffff'
  error-container: '#ffdad6'
  on-error-container: '#93000a'
  primary-fixed: '#dbe1ff'
  primary-fixed-dim: '#b4c5ff'
  on-primary-fixed: '#00174b'
  on-primary-fixed-variant: '#003ea8'
  secondary-fixed: '#85f8c4'
  secondary-fixed-dim: '#68dba9'
  on-secondary-fixed: '#002114'
  on-secondary-fixed-variant: '#005137'
  tertiary-fixed: '#dae2fd'
  tertiary-fixed-dim: '#bec6e0'
  on-tertiary-fixed: '#131b2e'
  on-tertiary-fixed-variant: '#3f465c'
  background: '#f8f9ff'
  on-background: '#0b1c30'
  surface-variant: '#d3e4fe'
  surface-canvas: '#F8FAFC'
  surface-panel: '#FFFFFF'
  surface-muted: '#F1F5F9'
  surface-recessed: '#EDF2F7'
  surface-terminal: '#FAFCFF'
  border-subtle: '#E2E8F0'
  border-strong: '#CBD5E1'
  border-active: '#2563EB'
  text-primary: '#0F172A'
  text-secondary: '#334155'
  text-muted: '#64748B'
  syntax-blue: '#2563EB'
  syntax-green: '#059669'
  syntax-amber: '#D97706'
  syntax-rose: '#E11D48'
  syntax-purple: '#7C3AED'
  syntax-cyan: '#0284C7'
typography:
  headline-lg:
    fontFamily: Geist
    fontSize: 1.25rem
    fontWeight: '600'
    lineHeight: 1.75rem
    letterSpacing: -0.02em
  headline-sm:
    fontFamily: Geist
    fontSize: 1rem
    fontWeight: '600'
    lineHeight: 1.375rem
    letterSpacing: -0.015em
  body-default:
    fontFamily: Geist
    fontSize: 0.8125rem
    fontWeight: '400'
    lineHeight: 1.25rem
  body-medium:
    fontFamily: Geist
    fontSize: 0.8125rem
    fontWeight: '500'
    lineHeight: 1.25rem
  body-compact:
    fontFamily: Geist
    fontSize: 0.75rem
    fontWeight: '400'
    lineHeight: 1.125rem
  code-terminal:
    fontFamily: JetBrains Mono
    fontSize: 0.8125rem
    fontWeight: '400'
    lineHeight: 1.375rem
    letterSpacing: -0.005em
  code-dense:
    fontFamily: JetBrains Mono
    fontSize: 0.75rem
    fontWeight: '400'
    lineHeight: 1.125rem
  label-ui:
    fontFamily: Geist
    fontSize: 0.6875rem
    fontWeight: '500'
    lineHeight: 0.875rem
    letterSpacing: 0.01em
  label-mono:
    fontFamily: JetBrains Mono
    fontSize: 0.6875rem
    fontWeight: '500'
    lineHeight: 0.875rem
    letterSpacing: 0.02em
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

This design system is engineered for systems engineers, site reliability engineers (SREs), cloud architects, and advanced terminal power users operating in high-clarity daylight or illuminated workstation environments. It translates the dense utility of mission-critical terminal workbenches—such as Xshell, iTerm2, and VS Code—into a refined, pristine desktop aesthetic inspired by macOS HIG and contemporary developer workbenches.

The visual style unites **Technical Minimalism** with **Pragmatic Desktop Precision**:
- High-density information architecture designed to maximize viewport economy across multi-pane SSH sessions, SFTP transfer queues, and Kubernetes telemetry.
- Architectural clarity built around layered off-white and cool paper surfaces, 1px structural division lines, and razor-sharp contrast.
- Deep slate and carbon typography engineered for zero optical fatigue during continuous 8-to-12-hour monitoring shifts.
- Rich syntax accents—electric indigo, terminal emerald, warm amber, and vivid rose—that preserve instant semantic scanning without visual clutter.

## Colors

The color system delivers high legibility through layered off-white and neutral paper planes, stabilized by deep slate neutrals and punctated by ANSI-derived developer syntax tones.

### Canvas & Surface Architecture
- **Terminal & Shell Canvas (`#FAFCFF`)**: An ultra-clean, faintly cooled terminal paper canvas calibrated for crisp monospace rendering with dark text ink.
- **Base Scaffold Canvas (`#F8FAFC`)**: Outermost app container, tool rails, and window header backgrounds.
- **Active Pane & Document Canvas (`#FFFFFF`)**: Pure white primary workspace for active SSH split-screens, editor tabs, and data tables.
- **Secondary Chrome & Sidebar (`#F1F5F9`)**: Tree views, session catalogs, inspector sidebars, and inactive pane tabs.
- **Recessed Inset Canvas (`#EDF2F7`)**: Recessed status bars, search inputs, command drawers, and scroll track channels.

### Text & Ink Layers
- **Primary Text (`#0F172A`)**: Deep slate black for command outputs, prompts, active tabs, and primary headers.
- **Secondary Text (`#334155`)**: Mid-slate for file paths, hostnames, column headers, and active UI controls.
- **Muted Text (`#64748B`)**: Timestamps, connection protocol tags, inactive tabs, and breadcrumb dividers.

### Borders & Separators
- **Structural Partition (`#E2E8F0`)**: Hairline 1px borders between split panes, tool panels, and tab boundaries.
- **Interactive Outline (`#CBD5E1`)**: Hover states, unfocused input fields, and drag sashes.
- **Accent Focus Line (`#2563EB`)**: Active pane highlight, primary keyboard focus ring, and active tab indicator strips.

### Operational Accents & Syntax
- **Indigo (`#2563EB`)**: Primary interactive states, active pane focus, prompt directories, and SSH tunnel connections.
- **Emerald (`#059669`)**: Live status, open ports, successful exit codes (`0`), and active tunnel heartbeat indicators.
- **Amber (`#D97706`)**: Elevated privileges (`sudo`), warnings, high CPU/RAM memory flags, and dirty git buffers.
- **Rose (`#E11D48`)**: Closed sockets, SSH timeouts, process terminations, and error streams (`stderr`).
- **Purple (`#7C3AED`)**: Environment keys, variable interpolations, and shell function identifiers.
- **Cyan (`#0284C7`)**: IPv4/IPv6 address blocks, SFTP symlinks, and network transfer velocity badges.

## Typography

The typography architecture enforces strict dual-family specialization: **Geist** for high-clarity application scaffolding, window headers, tree views, and modal settings; **JetBrains Mono** for raw terminal emulation, shell commands, hexadecimal viewports, telemetry tables, and connection matrices.

- **Proportional UI Baseline**: Application body text is set to 13px (`0.8125rem`), maximizing vertical session density without sacrificing optical distinction between glyphs.
- **Monospace Optical Balance**: Terminal viewports run JetBrains Mono at 13px (`0.8125rem`) with a generous 1.375rem line-height, allowing rapid line-by-line scanning of continuous server logs and stack traces.
- **Tabular Figures & Metrics**: Numeric telemetry (bandwidth, latency, ping, port numbers) always defaults to `label-mono` to prevent horizontal jitter during real-time polling updates.

## Layout & Spacing

The layout model is governed by a **Docked Pane & Split Grid** architecture calibrated for multi-session terminal workbenches. The layout prioritizes window density and fluid resizing over decorative margins.

### Layout Model
- **Tool Rail (Activity Bar)**: Fixed 40px width on the left-most flank for direct toggling of Sessions, Snippets, Port Forwarding, and Settings.
- **Host / Session Explorer**: Collapsible panel (default 240px width, min 180px, max 420px) containing server trees and cluster groups.
- **Terminal Workspace Canvas**: Multi-pane split grid (horizontal and vertical splits) stretching to 100% of available viewport width and height.
- **Splitter Sashes**: 3px wide invisible drag handles with a centered 1px structural separator line (`#E2E8F0`). On drag/hover, the sash expands visually to 2px with an active tint of `#2563EB`.

### Spacing Principles
- UI components are built around a tight 4px baseline rhythm.
- Monospace terminal padding uses `space-md` (8px) along left and top margins, ensuring character buffers avoid touching physical pane borders.
- Host tree lines and tabular session lists maintain a compact 24px vertical row height to present up to 35 visible sessions per column without scrolling.

## Elevation & Depth

Visual hierarchy is communicated through **structural 1px borders** and **tonal panel stratification**, rather than soft blur shadows. This avoids rendering overhead during rapid terminal text reflow and maintains razor-sharp window boundaries.

### Structural Depth Tiers
- **Recessed Tier (Status Bars, Path Bar, Inputs)**: Surface `#EDF2F7` with inner 1px border `#E2E8F0`.
- **Base Tier (Shell Canvas & Terminal Panes)**: Surface `#FAFCFF` or `#FFFFFF` framed by a clean 1px border `#E2E8F0`.
- **Scaffold Tier (Sidebars, Window Rails, Tab Strips)**: Surface `#F1F5F9` and `#F8FAFC`.
- **Floating Overlays (Command Palette, Autocomplete Menus, Context Menus)**: Surface `#FFFFFF` enclosed by a crisp 1px `#CBD5E1` outline and an ambient desktop shadow: `0 4px 16px -2px rgba(15, 23, 42, 0.08), 0 2px 6px -1px rgba(15, 23, 42, 0.04)`.

### Focus & Active State Indication
The currently active terminal split pane receives an active 1px perimeter border in `#2563EB` and a solid 2px accent line atop its corresponding tab. Inactive terminal splits carry a 1px border in `#E2E8F0` with a subtle, non-intrusive 2% surface tinting of `#F8FAFC` to signal passive state.

## Shapes

The shape system employs a **Soft (Level 1)** geometric form factor to match contemporary professional desktop IDEs and native macOS workstation utilities.

- **Standard Controls (Inputs, Push Buttons, Segmented Controls)**: 4px (`0.25rem`) corner radius.
- **Workstation Split Tiles**: 0px radius at all interior split junctures, ensuring flush, non-overlapping seam lines across high-density tiled windows.
- **Floating Modals, Dialogs & Command Palettes**: 6px (`0.375rem`) to 8px (`0.5rem`) corner radius (`rounded-lg`) to set floating utility surfaces apart from the docked workstation grid beneath them.
- **Terminal Status Chips & Badges**: 3px (`0.1875rem`) corner radius, maintaining a precise, technical look.

## Components

### Tab Bars & Session Tabs
- **Tab Bar Strip**: Height 32px, background `#F1F5F9`, border-bottom 1px solid `#E2E8F0`, zero padding.
- **Inactive Tab**: Background `#F1F5F9`, text `#64748B`, border-right 1px solid `#E2E8F0`. Hover transitions text to `#0F172A` and background to `#F8FAFC`.
- **Active Tab**: Background `#FFFFFF`, text `#0F172A`, font-weight 500, with a 2px top accent line in `#2563EB`.
- **Status Dot Indicator**: 6px circular dot positioned 6px before tab title: `#059669` (Connected), `#D97706` (Transferring / Heavy Load), `#E11D48` (Disconnected / Error).
- **Close Action**: 14px x 14px glyph, text `#64748B`, hover background `#E2E8F0`, hover text `#E11D48`, border-radius 2px.

### Buttons & Quick Actions
- **Primary Button**: Height 26px, padding `0 10px`, background `#2563EB`, text `#FFFFFF`, font-size 12px, font-weight 500, border-radius 4px. Hover `#1D4ED8`, active `#1E40AF`.
- **Secondary / Toolbar Button**: Height 24px, padding `0 8px`, background `#FFFFFF`, border 1px solid `#CBD5E1`, text `#334155`. Hover background `#F8FAFC`, border `#94A3B8`.
- **Compact Tool Icon**: 22px x 22px square, background `transparent`, text `#64748B`. Hover background `#E2E8F0`, text `#0F172A`.

### Input Fields & Terminal Find Bar
- **Input Field**: Height 26px, background `#FFFFFF`, border 1px solid `#CBD5E1`, text `#0F172A`, font-family `JetBrains Mono`, font-size 12px, padding `0 6px`, border-radius 3px.
- **Focus State**: Border-color `#2563EB` with an ambient ring: `box-shadow: 0 0 0 1px #2563EB`.
- **Search Toggles (Regex, Match Case, Whole Word)**: 18px x 18px inline buttons within the search container. Inactive: text `#64748B`. Active: background `#DBEAFE`, text `#2563EB`, font-weight 600.

### Tree Views (Host Catalog & SFTP File Browser)
- **Item Row**: Height 22px, font-family `Geist`, font-size 12px, padding `0 6px`, cursor pointer.
- **Indent Guildelines**: 1px solid `#E2E8F0` positioned at 12px indentation increments.
- **Hover Row**: Background `#F1F5F9`.
- **Selected Row**: Background `#EFF6FF`, text `#2563EB`, font-weight 500, with a 2px left border strip in `#2563EB`.
- **Protocol Badges**: Monospace uppercase pill (`SSH`, `RDP`, `SFTP`), font-size 9px, padding `1px 4px`, background `#F1F5F9`, border 1px solid `#E2E8F0`, text `#0284C7`.

### Terminal Viewport & Prompt Display
- **Viewport Canvas**: Background `#FAFCFF`, text `#0F172A`, font-family `JetBrains Mono`.
- **Cursor**: 2px solid block or beam in `#2563EB`, with a 1s smooth blink interval.
- **Selection Highlight**: `background: #BFDBFE`, preserving text color readability in `#0F172A`.

### Command Palette & Quick Switcher
- **Modal Container**: Width 540px, centered top (15% offset), background `#FFFFFF`, border 1px solid `#CBD5E1`, border-radius 6px, elevation shadow.
- **Search Input**: Height 36px, font-family `Geist`, font-size 14px, border-bottom 1px solid `#E2E8F0`, padding `0 12px`.
- **Keyboard Shortcut Kbd**: Background `#F1F5F9`, border 1px solid `#CBD5E1`, border-bottom 2px solid `#94A3B8`, border-radius 3px, font-family `JetBrains Mono`, font-size 10px, padding `1px 4px`, text `#475569`.