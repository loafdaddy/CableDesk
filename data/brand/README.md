# CableDesk brand

Visual direction: dark, calm, copper accent — direct cable link between two machines.
Sibling feel to Cadence / Discoverr / spotDL (dark mark + accent period), distinct palette.

| File | Use |
|------|-----|
| `cabledesk-mark.svg` | Icon / avatar / favicon-style mark |
| `cabledesk-lockup.svg` | README and marketing (“CableDesk.”) |
| `cabledesk-social-banner.svg` | Social / Open Graph style banner |
| `../icons/hicolor/scalable/apps/org.cabledesk.CableDesk.svg` | Desktop app icon |

## Palette

| Token | Hex | Role |
|-------|-----|------|
| Accent | `#E8A45C` | Period, cable strokes, highlights |
| Accent soft | `#F5C48A` | Lighter cable stop |
| Accent deep | `#C47E38` | Darker cable stop |
| Deep | `#1A120A` | Mark background mid-stop |
| Deep top | `#2A1C10` | Mark background highlight |
| Deep bottom | `#0E0A06` | Mark background shadow |
| Soft text | `#FFF6EB` | Wordmark |

Wordmark ends with a copper period at normal font spacing.

## Typography

Lockup wordmark is **Cantarell Extra Bold** (GNOME’s classic UI face), outlined as SVG paths so GitHub and other hosts render the same weight without needing the font installed.

The copper period uses the font’s normal advance after `CableDesk` (as in typed `CableDesk.`).

Fallback stack if you re-edit as live text: `Cantarell Extra Bold, Cantarell, Adwaita Sans, Inter, Segoe UI, Ubuntu, system-ui, sans-serif` at weight **800**.

## Usage notes

- Prefer the **lockup** in README heroes and marketing.
- Prefer the **mark** alone for app icons, About dialog, empty states, and square crops.
- Prefer the **social banner** for repository social previews and share cards.
- Do not recolor the accent to purple (Cadence), teal (Discoverr), green (spotDL), or sky blue (Rust Weather).
