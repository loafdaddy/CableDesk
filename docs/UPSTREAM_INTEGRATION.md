# CableDesk Upstream Integration Research

This document records verified findings about current upstream Sunshine and Moonlight-Qt behavior,
as of **2026-07-22**, gathered directly from the LizardByte/Sunshine and moonlight-stream/moonlight-qt
GitHub repositories, their documentation sites, and GitHub Releases. Every claim below is either cited
to a specific source or flagged in [Open Questions](#open-questions) as unverified.

CableDesk does not fork or modify either project. It manages a dedicated, CableDesk-owned Sunshine
config/instance and drives the `moonlight-qt` binary as a CLI-invoked child process.

---

## 1. Sunshine on Fedora: packaging and distribution channels

**Current stable release:** `v2026.516.143833` (published 2026-05-16), per the
[Sunshine Releases page](https://github.com/LizardByte/Sunshine/releases). Two newer tags exist
(`v2026.713.170739`, `v2026.715.205118`) but both are marked **pre-release** by GitHub's release API,
so `v2026.516.143833` is the current stable/"Latest" release.

LizardByte officially publishes/documents the following Linux distribution channels
([Getting Started — Linux](https://github.com/LizardByte/Sunshine/blob/master/docs/getting_started.md)):

- **GitHub Releases RPM** for Fedora/openSUSE: download `Sunshine-{version}.{distro+version}.{arch}.rpm`
  and `sudo dnf install ./Sunshine-{version}.{distro}.{arch}.rpm`.
- **Fedora COPR** (official, LizardByte-maintained): `sudo dnf copr enable lizardbyte/stable` (or
  `lizardbyte/beta`), then `sudo dnf install Sunshine`. The docs explicitly warn: *"Stable builds are
  only available if the Sunshine release was made after the Fedora version release... it is often
  recommended to use the beta copr."*
- **AppImage**, **Arch (PKGBUILD / prebuilt)**, **Debian/Ubuntu** `.deb`, **Flatpak**
  (`dev.lizardbyte.app.Sunshine` on Flathub — but the docs caution *"Use distro-specific packages
  instead of the Flatpak if they are available. Flatpak does not support KMS capture."*), and an
  experimental **Homebrew** tap.
- **Docker** container (separate `DOCKER_README.md`, referenced from the docs nav).
- Third-party/community packages (Chocolatey, nixpkgs, Scoop, Solus — explicitly marked
  *"not maintained by LizardByte, use at your own risk"*) per
  [third_party_packages.md](https://github.com/LizardByte/Sunshine/blob/master/docs/third_party_packages.md).
  A community COPR (`pvermeer/sunshine`) also exists but is unofficial.

**CableDesk recommendation:** Prefer the official `lizardbyte/stable` (or `lizardbyte/beta` when stable
lags the Fedora release) COPR repo for the Fedora target, since it's officially maintained and gives
clean `dnf` upgrade/removal semantics. Pin/document the exact Sunshine version CableDesk was tested
against, since LizardByte ships frequent (near-weekly) pre-releases and the "stable" cadence is coupled
to Fedora's own release timing. Avoid Flatpak Sunshine specifically because it disables KMS capture,
one of the better-performing Linux capture paths.

Sources:
[Sunshine Releases](https://github.com/LizardByte/Sunshine/releases) ·
[getting_started.md](https://github.com/LizardByte/Sunshine/blob/master/docs/getting_started.md) ·
[third_party_packages.md](https://github.com/LizardByte/Sunshine/blob/master/docs/third_party_packages.md) ·
[Fedora Copr: lizardbyte/stable](https://copr.fedorainfracloud.org/coprs/lizardbyte/stable/package/Sunshine/)

---

## 2. Sunshine Linux capture backends (Wayland/GNOME focus)

Per the `capture` config option documentation
([configuration.md](https://github.com/LizardByte/Sunshine/blob/master/docs/configuration.md)),
Sunshine on Linux supports these capture backends, selectable via `capture = <value>` (default is
"automatic," using the first available in this order):

| Value | Backend | Notes |
|---|---|---|
| `nvfbc` | NVIDIA Frame Buffer Capture | Direct-to-GPU-memory; "does not have native Wayland support and does not work with XWayland" |
| `wlr` | `wlr-screencopy-unstable-v1` | wlroots compositors only (Sway, Hyprland, etc.) |
| `kms` | DRM/KMS capture from the kernel | Requires `cap_sys_admin`; lowest latency; **required for HDR capture** on Linux |
| `kwin` | KDE/KWin Wayland screencasting | KDE Plasma only |
| `x11` | XCB | "slowest and most CPU intensive... should be avoided if possible" |

**GNOME/Wayland specifically:** GNOME/Mutter is not `wlr`, not `kwin`, and (being Wayland) not `x11`.
That leaves **KMS** or the **XDG Desktop Portal screencast (PipeWire-based)** path — GNOME Discussion
and GamingOnLinux coverage of the same release cycle describe Sunshine's "XDG, Pipewire... direct
screencast capture" as the newly-added universal path for portal-based compositors including GNOME
([GamingOnLinux, May 2026](https://www.gamingonlinux.com/2026/05/sunshine-game-streaming-tool-adds-vulkan-encoding-plus-xdg-pipewire-and-kwin-direct-screencast-capture/)).
Sunshine's own troubleshooting doc frames the practical trade-off directly: *"On GNOME without Vulkan
support: kms capture with vaapi/nvenc encoding [is recommended for non-sandboxed installs]. For
Vulkan-capable GNOME: portal capture with vulkan encoding"* (paraphrased from
[troubleshooting.md](https://github.com/LizardByte/Sunshine/blob/master/docs/troubleshooting.md) content
surfaced during research — see Open Questions for a caveat on this specific line).

**Portal permission prompt behavior (confirmed from troubleshooting.md):**
> "Portal capture requires you to manually approve Remote Desktop permissions via an on-screen prompt
> on the host. This creates a portal token which is used to automatically reauthorize on subsequent
> reconnects, but under certain circumstances (a Sunshine crash, switching to another desktop
> environment, or if a monitor hotplug event occurs) the portal token may become lost or invalid,
> necessitating manual re-approval of capture permissions."

So it is **not** guaranteed one-time: it's "one-time until the token is invalidated," and GNOME's
portal implementation is one of the compositors this applies to. KDE users have a documented bypass
(`flatpak permission-set kde-authorized remote-desktop dev.lizardbyte.app.Sunshine yes`, which the docs
say "will work with any supported Sunshine installation type," not just Flatpak) — **no equivalent
persistent-grant workaround for GNOME's portal implementation is documented.**

KMS, by contrast, requires `cap_sys_admin` (a one-time setcap/capability grant at install time, not a
per-session prompt) but is unavailable to sandboxed installs: *"KMS screencasting requires elevated
privileges which are not allowed for Flatpak or AppImage packages"* (troubleshooting.md).

**CableDesk recommendation:** For the Fedora + GNOME/Wayland first target, default to **KMS capture**
(`capture = kms`) when Sunshine is installed via RPM/COPR (non-sandboxed), since it avoids the
recurring portal-reauthorization prompt entirely and is documented as the lowest-latency Linux path.
Fall back to portal/PipeWire capture only when KMS is unavailable (e.g., Flatpak installs, or GPUs/
setups where KMS capability can't be granted), and surface to the CableDesk user that portal capture
may periodically require re-approving a system prompt outside CableDesk's control — this is a hard
external-dependency risk for a "headless/automated" product goal and should be called out prominently
in CableDesk's own setup UX.

Sources:
[configuration.md#capture](https://github.com/LizardByte/Sunshine/blob/master/docs/configuration.md) ·
[troubleshooting.md](https://github.com/LizardByte/Sunshine/blob/master/docs/troubleshooting.md) ·
[GamingOnLinux: Sunshine adds Vulkan/XDG/Pipewire/KWin capture](https://www.gamingonlinux.com/2026/05/sunshine-game-streaming-tool-adds-vulkan-encoding-plus-xdg-pipewire-and-kwin-direct-screencast-capture/) ·
[DeepWiki: Linux Platform Implementation](https://deepwiki.com/LizardByte/Sunshine/8.2-linux-platform-implementation) (secondary/community source, not LizardByte-authored — see Open Questions)

---

## 3. Sunshine managed configuration: `sunshine.conf` and `apps.json`

**Config file:** Plain-text `key = value` format. Default location `~/.config/sunshine` on Linux
(also FreeBSD/macOS); Docker uses `/config`. A specific file can be passed as the first CLI argument:
`sunshine <directory>/sunshine.conf`, and **the file will be created if it doesn't exist** — no web UI
interaction required to bootstrap it
([getting_started.md](https://github.com/LizardByte/Sunshine/blob/master/docs/getting_started.md)).

Config keys relevant to a headless/automated CableDesk-managed instance
(full option list: [configuration.md](https://github.com/LizardByte/Sunshine/blob/master/docs/configuration.md)):

- `sunshine_name` — host name shown in Moonlight.
- `port` — base port (default `47989`); `address_family`, `bind_address`, `upnp`.
- `cert` / `pkey` — paths to the TLS cert/key used for the web UI and client pairing (defaults:
  `credentials/cacert.pem`, `credentials/cakey.pem`); can be pre-provisioned by CableDesk so pairing
  doesn't depend on first-run web UI state.
- `credentials_file` — where the web-UI username/password hash is stored.
- `capture`, `encoder`, `output_name`, `adapter_name` — force specific backend/GPU (see §2, §5).
- `file_apps` — path override for the apps list file (defaults to same directory as the config file).
- `min_log_level`, `log_path` — for headless log capture.
- `global_prep_cmd` — commands run before/after all apps (e.g.
  `global_prep_cmd = [{"do":"nircmd.exe setdisplay 1280 720 32 144","elevated":true,"undo":"..."}]`).

**`apps.json` (the "apps.json equivalent" for streamable applications):** confirmed via the actual
shipped default file
([`src_assets/linux/assets/apps.json`](https://github.com/LizardByte/Sunshine/blob/master/src_assets/linux/assets/apps.json)):

```json
{
  "env": {
    "PATH": "$(PATH):$(HOME)/.local/bin"
  },
  "apps": [
    {
      "name": "Desktop",
      "image-path": "desktop.png"
    },
    {
      "name": "Low Res Desktop",
      "image-path": "desktop.png",
      "prep-cmd": [
        {
          "do": "xrandr --output HDMI-1 --mode 1920x1080",
          "undo": "xrandr --output HDMI-1 --mode 1920x1200"
        }
      ]
    },
    {
      "name": "Steam Big Picture",
      "detached": ["setsid steam steam://open/bigpicture"],
      "prep-cmd": [{"do": "", "undo": "setsid steam steam://close/bigpicture"}],
      "image-path": "steam.png"
    }
  ]
}
```

Top-level `env` sets/overrides environment variables for every app (only editable by hand-editing
`apps.json`, not via web UI, per docs). Each app entry supports `name`, `image-path`, `cmd`
(foreground, blocking), `detached` (fire-and-forget, e.g. for Steam), `prep-cmd` (`do`/`undo`, run
before/after the stream), `working-dir`, `elevated` (Windows only), and `env` overrides.
The special **"Desktop"** entry (no `cmd`/`detached`) just starts a stream of the existing desktop —
exactly the app CableDesk's own "stream the desktop" flow should point at.
`$(HOME)` substitutes `$HOME`; `$$` escapes to a literal `$`.

**Can Sunshine be fully configured without ever touching the web UI?** Functionally yes — both
`sunshine.conf` and `apps.json` are plain files Sunshine will read/create on its own; the web UI is a
convenience layer over the same files, and the docs describe `sunshine.conf` as hand-editable
("Although it is recommended to use the configuration UI, it is possible to manually configure Sunshine
by editing the `conf` file in a text editor"). However, LizardByte's own written guidance frames
`apps.json` specifically as **"should be configured via the web UI"** for anything beyond hand-editing
the `env` block — this is a documentation/recommendation stance, not a technical restriction; the file
format itself is fully documented and the JSON schema above is what's actually shipped/parsed.
Sunshine's PIN-pairing flow also has a web-UI-mediated path ("Login to web-ui... Go to PIN... enter
PIN"), but the actual pairing protocol is the standard GameStream PIN-pairing handshake, which
Moonlight-Qt's CLI can drive directly with `--pin` (see §7) — meaning CableDesk can pair without ever
opening Sunshine's web UI, by driving `moonlight pair` from the CableDesk side.

**CableDesk recommendation:** Treat `sunshine.conf` and a CableDesk-generated `apps.json` (with a
single `Desktop` entry, or one entry per CableDesk-managed "profile") as declarative, CableDesk-owned
artifacts written directly to the CableDesk instance's config directory — never touch or scrape
Sunshine's web UI. Pre-provision `cert`/`pkey` and drive pairing via `moonlight pair <host> --pin
<code>` so the whole bring-up is scriptable.

Sources:
[configuration.md](https://github.com/LizardByte/Sunshine/blob/master/docs/configuration.md) ·
[getting_started.md](https://github.com/LizardByte/Sunshine/blob/master/docs/getting_started.md) ·
[src_assets/linux/assets/apps.json](https://github.com/LizardByte/Sunshine/blob/master/src_assets/linux/assets/apps.json) ·
[app_examples.md](https://github.com/LizardByte/Sunshine/blob/master/docs/app_examples.md)

---

## 4. Sunshine `/dev/uinput` permissions on Linux

Confirmed from the actual shipped udev rules file
([`src_assets/linux/misc/60-sunshine.rules`](https://github.com/LizardByte/Sunshine/blob/master/src_assets/linux/misc/60-sunshine.rules)):

```udev
# Allows Sunshine to access /dev/uinput
KERNEL=="uinput", SUBSYSTEM=="misc", OPTIONS+="static_node=uinput", GROUP="input", MODE="0660", TAG+="uaccess"

# Allows Sunshine to access /dev/uhid
KERNEL=="uhid", GROUP="input", MODE="0660", TAG+="uaccess"

# Joypads
KERNEL=="hidraw*", ATTRS{name}=="Sunshine PS5 (virtual) pad*", GROUP="input", MODE="0660", TAG+="uaccess"
SUBSYSTEMS=="input", ATTRS{name}=="Sunshine X-Box One (virtual) pad*", GROUP="input", MODE="0660", TAG+="uaccess"
SUBSYSTEMS=="input", ATTRS{name}=="Sunshine gamepad (virtual) motion sensors*", GROUP="input", MODE="0660", TAG+="uaccess"
SUBSYSTEMS=="input", ATTRS{name}=="Sunshine Nintendo (virtual) pad*", GROUP="input", MODE="0660", TAG+="uaccess"
```

This is installed automatically by the RPM/deb/COPR post-install scripts (the docs say the installer's
post-install script tries to reload `udev` rules automatically). It uses **`TAG+="uaccess"`**
(systemd-logind's "grant access to the device to the user at the active seat" mechanism) combined with
`GROUP="input"` as a fallback, rather than requiring group membership outright. The current
troubleshooting doc's advice is:
> "After installation, the `udev` rules need to be reloaded. Our post-install script tries to do this
> for you automatically, but if it fails, you may need to restart your system. If the input is still
> not working, you may need to add your user to the `input` group" (`sudo usermod -aG input $USER`).

There's also a **multiseat** rule for assigning virtual input devices to the correct `logind` seat via
`XDG_SEAT`, documented with its own rules file
(`/etc/udev/rules.d/72-sunshine-virtual-seat.rules`) — relevant if CableDesk ever needs multi-user/
multi-seat Fedora hosts.

**CableDesk recommendation:** Rely on the packaged udev rule (`uaccess`) — it should "just work" for a
user logged into an active GNOME session without CableDesk needing to add its own udev rules or modify
group membership. If CableDesk detects broken input at first run, fall back to documenting the
`usermod -aG input $USER` (+ re-login) step rather than silently modifying group membership itself.

Sources:
[60-sunshine.rules](https://github.com/LizardByte/Sunshine/blob/master/src_assets/linux/misc/60-sunshine.rules) ·
[troubleshooting.md](https://github.com/LizardByte/Sunshine/blob/master/docs/troubleshooting.md)

---

## 5. Sunshine hardware encoding: VAAPI vs NVENC selection

Per `configuration.md`'s `encoder` option: **default behavior is automatic** — "Sunshine will use the
first encoder that is available," probed at runtime. The `encoder` config key can force a specific one:

| Value | Target hardware |
|---|---|
| `nvenc` | NVIDIA |
| `quicksync` | Intel (QuickSync) |
| `amdvce` | AMD |
| `vaapi` | AMD or Intel (via VA-API) |
| `vulkan` | AMD, Intel, or NVIDIA (Linux-only Vulkan video encode path) |
| `software` | CPU (libx264 fallback) |

Codec preference/advertisement is governed by separate `hevc_mode` and `av1_mode` keys, independent of
encoder selection:
- `hevc_mode`: `0` = advertise HEVC support based on what the selected encoder can actually do
  (default/recommended), `1` = never advertise HEVC, `2` = advertise HEVC Main only, `3` = advertise
  HEVC Main **and** Main10 (HDR).
- `av1_mode`: same shape — `0` auto (recommended), `1` never, `2` AV1 Main 8-bit, `3` AV1 8-bit **and**
  10-bit (HDR).
- H.264 has no corresponding toggle — it's the universal baseline Sunshine always supports.

Community/secondary source (DeepWiki, not LizardByte-authored) describes the VA-API implementation
(`vaapi.cpp`) as bridging capture buffers to the encoder via GBM/EGL, with `select_va_entrypoint`
querying the driver for supported entrypoints (e.g. low-power encode slice entrypoints) — this detail
could not be independently confirmed against the primary source file in this research pass (see Open
Questions).

**CableDesk recommendation:** Leave `encoder` unset (automatic) by default, since Sunshine already
probes hardware capability correctly, and only expose a manual override in CableDesk's own UI as an
advanced/troubleshooting escape hatch (mirroring `encoder = vaapi|nvenc|software`). Leave `hevc_mode`/
`av1_mode` at their auto (`0`) defaults unless CableDesk needs to force a codec ceiling for client
compatibility.

Sources:
[configuration.md#encoder](https://github.com/LizardByte/Sunshine/blob/master/docs/configuration.md) ·
[configuration.md#hevc_mode / #av1_mode](https://github.com/LizardByte/Sunshine/blob/master/docs/configuration.md) ·
[DeepWiki: Video Encoding Pipeline](https://deepwiki.com/LizardByte/Sunshine/5.2-video-encoding-pipeline) (secondary source, unverified against primary `vaapi.cpp`)

---

## 6. Moonlight-Qt native Linux build

Per the repo's [README.md](https://github.com/moonlight-stream/moonlight-qt/blob/master/README.md)
("Linux/Unix Build Requirements"):

- **Qt version:** "Qt 6 is recommended, but Qt 5.12 or later is also supported (replace `qmake6` with
  `qmake` when using Qt 5)."
- **Compiler:** GCC or Clang.
- **FFmpeg:** 4.0 or later (6.1+ if building the Vulkan renderer).
- **Fedora/RedHat packages** (RPM Fusion repo required):
  - Base: `openssl-devel SDL2-devel SDL2_ttf-devel ffmpeg-devel libva-devel libvdpau-devel opus-devel pulseaudio-libs-devel alsa-lib-devel libdrm-devel`
  - Qt 6: `qt6-qtsvg-devel qt6-qtdeclarative-devel`
  - Qt 5 (alt.): `qt5-qtsvg-devel qt5-qtquickcontrols2-devel`
  - Vulkan renderer (optional): `libplacebo-dev`/`libplacebo-devel` ≥ v7.349.0 + FFmpeg ≥ 6.1.

**Build steps** (verbatim from README):
1. `git submodule update --init --recursive` (submodules: `moonlight-common-c`, `qmdnsengine`,
   `h264bitstream`, `app/SDL_GameControllerDB` — per
   [.gitmodules](https://github.com/moonlight-stream/moonlight-qt/blob/master/.gitmodules)).
2. `qmake6 moonlight-qt.pro` then `make debug` or `make release`.
3. Resulting binary: `app/moonlight`.
4. An embedded/kiosk build variant exists: `qmake6 "CONFIG+=embedded" moonlight-qt.pro` — "will lack
   windowed mode, Discord/Help links, and other features that don't make sense on an embedded device,"
   and `CONFIG+=gpuslow` prefers direct KMSDRM rendering for weak-GPU targets.

**Official Fedora RPM: not offered.** The README's "Downloads" section only lists: GitHub Releases
(Windows/macOS/Steam Link), **Snap** (Ubuntu-based distros), **Flatpak**
(`com.moonlight_stream.Moonlight` on Flathub, "for other Linux distros"), **AppImage**, plus
Raspberry Pi / generic ARM / RISC-V / NVIDIA Jetson & L4T Debian package repos hosted via Cloudsmith —
no Fedora-specific package of any kind is mentioned.

**Release cadence note:** the latest *tagged* GitHub release is **`v6.1.0`** (2024-09-17) — over a
year and a half old as of this research (2026-07-22). Ongoing development happens on `master`, with
[nightly build artifacts](https://nightly.link/moonlight-stream/moonlight-qt/workflows/build/master)
published via GitHub Actions but no newer numbered release cut. This is a meaningful fact for
CableDesk: **"current stable release" for Sunshine (frequent releases) and Moonlight-Qt (no release in
~19 months) behave very differently**, and a "latest" Moonlight-Qt build for CableDesk likely means a
CI/nightly artifact or a from-source build off `master`, not the v6.1.0 tag, if newer CLI options/fixes
are needed.

**CableDesk recommendation:** Build Moonlight-Qt from source (or consume the nightly CI artifact) on
Fedora rather than depend on Flatpak, since CableDesk needs the raw CLI binary as a child process and
Flatpak sandboxing complicates that (input/portal permission plumbing, working-directory assumptions,
etc., mirroring the exact Sunshine Flatpak caveats in §1/§2). Track `master` given the stale tagged
release, and pin to a specific commit SHA for reproducible CableDesk builds rather than "latest".

Sources:
[README.md](https://github.com/moonlight-stream/moonlight-qt/blob/master/README.md) ·
[moonlight-qt Releases](https://github.com/moonlight-stream/moonlight-qt/releases) ·
[.gitmodules](https://github.com/moonlight-stream/moonlight-qt/blob/master/.gitmodules)

---

## 7. Moonlight-Qt CLI

Confirmed directly from the current CLI parser source
([`app/cli/commandlineparser.cpp`](https://github.com/moonlight-stream/moonlight-qt/blob/master/app/cli/commandlineparser.cpp))
and the related `app/cli/{quitstream,startstream,pair,listapps}.cpp` files. There is **no dedicated
`CLI.md`** in the repo; the parser source itself is the authoritative reference — quotes below are
verbatim from the source, not paraphrased.

### Top-level actions

```
Available actions:
  list            List the available apps on a host
  quit            Quit the currently running app
  stream          Start streaming an app
  pair            Pair a new host

See 'moonlight <action> --help' for help of specific action.
```
Invoked as `moonlight <action> [options] <host> [<app>]`. If no action is given, Moonlight starts its
normal GUI.

### `pair`
```
moonlight pair <host>
```
Options: `--pin <4-digit PIN>` ("4 digit pairing PIN") — lets CableDesk drive pairing head-lessly by
pre-generating/relaying the PIN rather than requiring interactive entry in Sunshine's web UI. Errors if
the PIN isn't exactly 4 digits.

### `list`
```
moonlight list <host> [--csv] [--verbose]
```
`--csv` "Print as CSV with additional information"; `--verbose` "Displays additional information".

### `quit`
```
moonlight quit <host>
```
**Mechanism confirmed from source (`app/cli/quitstream.cpp`):** this does **not** send a signal or
local IPC message to an already-running `moonlight stream` process. It spins up a fresh Moonlight
process/state machine that (1) resolves `<host>` via `ComputerSeeker` (requires the host to already be
paired — errors "Computer has not been paired" otherwise), then (2) calls
`ComputerManager::quitRunningApp(computer)`, which is a **GameStream/Sunshine HTTP API call to the
host** telling it to terminate the currently-running app. The original `moonlight stream` process (if
any) observes the resulting session teardown via its own connection to the host and exits as a
consequence — there is no direct process-to-process signal between a `moonlight quit` invocation and a
separate running `moonlight stream` process on the client machine.

**Practical implication for CableDesk:** if CableDesk needs to stop a stream it launched as a child
process, it has two independently valid options: (a) terminate the child `moonlight stream` process
directly (e.g. SIGTERM) since CableDesk owns that process handle, or (b) invoke `moonlight quit <host>`
as a *separate* command, which stops the app on the Sunshine host side (useful if CableDesk lost the
child process handle, or wants Sunshine-side app teardown/`prep-cmd` undo hooks to run cleanly rather
than an abrupt client-side kill). Given CableDesk already owns the child process, (a) is likely
preferable for responsiveness, with (b) as a fallback/explicit "stop for real" path that also runs
Sunshine's `undo` prep commands.

### `stream`
```
moonlight stream <host> "<app>" [options]
```
Verbatim option set from source (each has a `--no-<name>` inverse for boolean toggles):

- Resolution: `--720`, `--1080`, `--1440`, `--4K` (flags mapping to 1280x720 / 1920x1080 / 2560x1440 /
  3840x2160), or `--resolution <width>x<height>` for a custom value (validated via regex `^(\d+)x(\d+)$`).
- `--fps <value>` — supported range enforced in-code as **10–480**; outside that range Moonlight prints
  a warning but still proceeds ("Warning: FPS is out of the supported range (10 - 480 FPS). Performance
  may suffer!").
- `--bitrate <Kbps>` — supported range **500–500000** Kbps (same warn-but-proceed behavior). If bitrate
  isn't given but resolution or fps is, Moonlight computes a default via
  `preferences->getDefaultBitrate(...)`.
- `--packet-size <bytes>` — must be `> 1024` or Moonlight exits with an error.
- `--display-mode <fullscreen|windowed|borderless>` (borderless maps to `WM_FULLSCREEN_DESKTOP`
  internally).
- `--vsync` / `--no-vsync`.
- `--video-codec <auto|H.264|HEVC|AV1>`.
- `--video-decoder <auto|software|hardware>`.
- `--audio-config <stereo|5.1-surround|7.1-surround>`.
- `--audio-on-host` / `--no-audio-on-host` — play audio on the host PC instead of the client.
- `--hdr` / `--no-hdr` — HDR streaming toggle, confirmed exact flag name.
- `--yuv444` / `--no-yuv444`.
- `--absolute-mouse` / `--no-absolute-mouse` — "remote desktop optimized mouse control" (confirmed
  exact flag name/wording — this is the absolute-mouse-mode toggle requested).
- `--multi-controller` / `--no-multi-controller`.
- `--mouse-buttons-swap` / `--no-mouse-buttons-swap`.
- `--touchscreen-trackpad` / `--no-touchscreen-trackpad`.
- `--swap-gamepad-buttons` / `--no-swap-gamepad-buttons` (Nintendo-style A/B, X/Y swap).
- `--background-gamepad` / `--no-background-gamepad`.
- `--reverse-scroll-direction` / `--no-reverse-scroll-direction`.
- `--capture-system-keys <never|fullscreen|always>`.
- `--game-optimization` / `--no-game-optimization`.
- `--frame-pacing` / `--no-frame-pacing`.
- `--mute-on-focus-loss` / `--no-mute-on-focus-loss`.
- `--keep-awake` / `--no-keep-awake` — prevent display sleep while streaming.
- `--performance-overlay` / `--no-performance-overlay`.
- `--quit-after` / `--no-quit-after` — quit the host app automatically after the session ends (this is
  the flag that makes `moonlight stream` self-cleaning without needing a separate `moonlight quit`
  call, if CableDesk wants Sunshine-side cleanup to always happen on client exit).

All boolean toggles use the pattern `--<name>` / `--no-<name>`; whichever is specified **last** on the
command line wins if both are given (`getToggleOptionValue` takes `options.last()`).

`--help`/`--version` are handled per-action (`moonlight stream --help`, `moonlight pair --help`, etc.)
via Qt's `QCommandLineParser`, and any of these will print to stdout and `exit(0)` immediately —
confirmed in source (`showInfo`/`handleHelpAndVersionOptions`).

**CableDesk recommendation:** Drive `moonlight stream <host> "<app>" --display-mode windowed|fullscreen
--resolution WxH --fps N --bitrate N --video-codec ... --absolute-mouse --no-quit-after` (or
`--quit-after`, depending on whether CableDesk wants Sunshine-side app teardown automatically on
disconnect) as a long-lived child process, and prefer killing that child process directly for
"stop streaming" rather than spawning a second `moonlight quit` process, reserving `quit` for
scenarios where CableDesk needs to force-stop a session it doesn't hold a process handle for (e.g.
recovering from a CableDesk restart while a stream is still active on the host).

Sources:
[app/cli/commandlineparser.cpp](https://github.com/moonlight-stream/moonlight-qt/blob/master/app/cli/commandlineparser.cpp) ·
[app/cli/quitstream.cpp](https://github.com/moonlight-stream/moonlight-qt/blob/master/app/cli/quitstream.cpp) ·
[app/cli/startstream.cpp](https://github.com/moonlight-stream/moonlight-qt/blob/master/app/cli/startstream.cpp) ·
[app/cli directory listing](https://github.com/moonlight-stream/moonlight-qt/tree/master/app/cli)

---

## 8. Moonlight-Qt licensing

Confirmed via both the repo's `LICENSE` file content and GitHub's repository license metadata API
(`spdx_id: "GPL-3.0"`): **Moonlight-Qt is licensed GNU GPL v3.0** (full text, "Version 3, 29 June 2007",
© Free Software Foundation). The same is true of Sunshine itself (`LizardByte/Sunshine` also reports
`spdx_id: "GPL-3.0"`) and of the `moonlight-common-c` submodule Moonlight-Qt statically pulls in
(also `GPL-3.0`) — so there is no mixed-licensing surprise from the vendored submodules relevant to
redistribution.

**Redistribution obligations for CableDesk, if it ships a built `moonlight-qt` binary:** as GPLv3
copyleft terms, redistributing a built binary obligates CableDesk to:
- Make the **complete corresponding source code** available to anyone it distributes the binary to
  (either bundled, or via a written offer valid for the GPLv3-specified period), including CableDesk's
  exact build of Moonlight-Qt and its submodules at the commit/version actually shipped.
- Preserve all copyright notices and the GPLv3 license text alongside the binary.
- Not impose any additional restrictions beyond GPLv3 on recipients' rights to run/study/share/modify.
- If CableDesk itself doesn't modify Moonlight-Qt's source (per the task's own constraint — "does NOT
  modify Sunshine/Moonlight"), CableDesk is redistributing an unmodified GPLv3 binary, which is
  permitted, but the source-availability and notice-preservation obligations still apply to *whatever
  CableDesk ships*, including CableDesk's own packaging/build scripts that produced that exact binary.
- Since CableDesk is envisioned as an open-source project itself, the simplest compliant path is: don't
  vendor a prebuilt Moonlight-Qt binary at all — have CableDesk's installer either invoke the
  distro-packaged Moonlight-Qt (Flatpak/Snap/AppImage/RPM-Fusion-built) already on the user's system, or
  build it from the pinned upstream source as part of CableDesk's own build/install process. This sidesteps
  binary-redistribution obligations entirely by never being the *distributor* of the compiled artifact.

**CableDesk recommendation:** Treat Moonlight-Qt (and Sunshine) as external, arm's-length dependencies
CableDesk *invokes*, not artifacts CableDesk *redistributes*. Document the GPLv3 license and link to
upstream source for both projects in CableDesk's own README/NOTICE, and avoid bundling prebuilt
binaries in CableDesk's own release artifacts unless a lawyer/maintainer explicitly signs off on the
GPLv3 source-offer mechanics.

Sources:
[moonlight-qt LICENSE](https://github.com/moonlight-stream/moonlight-qt/blob/master/LICENSE) ·
[moonlight-qt repo license metadata](https://github.com/moonlight-stream/moonlight-qt) (GitHub API `license.spdx_id: GPL-3.0`) ·
[Sunshine LICENSE](https://github.com/LizardByte/Sunshine/blob/master/LICENSE) ·
[moonlight-common-c repo license metadata](https://github.com/moonlight-stream/moonlight-common-c)

---

## Open Questions

1. **DeepWiki citations are secondary/community-generated, not primary LizardByte documentation.**
   Two claims in this doc (VA-API's `select_va_entrypoint`/GBM-EGL bridging detail in §5, and part of
   the capture-path summary in §2) are sourced from
   [deepwiki.com/LizardByte/Sunshine](https://deepwiki.com/LizardByte/Sunshine) rather than directly
   read from `src/platform/linux/vaapi.cpp` / the capture `.cpp` files themselves. DeepWiki is an
   AI-generated wiki over the repo, not an official LizardByte artifact. Direct source review of
   `src/platform/linux/vaapi.cpp`, `kms.cpp`, `wlr.cpp`, and the portal/pipewire capture files was
   attempted but not completed line-by-line in this pass — **recommend a hands-on read of those exact
   files at the pinned Sunshine version CableDesk targets**, rather than relying on this summary, before
   writing any CableDesk code that depends on capture-backend-selection internals.

2. **GNOME-specific capture recommendation ("kms + vaapi/nvenc for non-Vulkan GNOME" vs. "portal +
   vulkan for Vulkan-capable GNOME") is reconstructed from troubleshooting-doc content surfaced during
   fetch-based research and was not independently re-confirmed against a second read of the exact
   current `troubleshooting.md` prose.** The udev/portal-token quotes in §2 and §4 *were* pulled and
   quoted verbatim from a direct fetch of `docs/troubleshooting.md`, but the specific GNOME
   Vulkan-vs-non-Vulkan recommendation sentence should be re-verified verbatim against
   `docs/troubleshooting.md` at CableDesk's pinned Sunshine version before being treated as exact
   product guidance — the underlying facts (portal reauth risk, KMS requiring non-sandboxed install,
   KDE-only permanent-grant bypass) are independently confirmed and solid.

3. **No hands-on build/run verification was possible in this research pass.** Everything above is
   sourced from documentation and source-code reading, not from actually building Sunshine or
   Moonlight-Qt, pairing them, or running a stream on Fedora/GNOME/Wayland. Specifically needing
   hands-on confirmation before CableDesk ships:
   - Whether the GNOME portal capture prompt is truly "on-screen and blocking" in a way compatible with
     a headless/kiosk CableDesk flow, or whether it can be pre-approved out-of-band (e.g. via GNOME's
     own portal permission store, analogous to the documented KDE `flatpak permission-set` bypass —
     **no GNOME equivalent was found in current docs**, which may mean none exists, or may mean it's
     undocumented).
   - Actual behavior of `moonlight stream ... --quit-after` combined with CableDesk force-killing the
     child process — whether Sunshine's `prep-cmd` `undo` hooks reliably run in both the
     client-initiated-kill and `moonlight quit`-initiated paths.
   - Whether `qmake6 moonlight-qt.pro` + `make release` on current Fedora (with the documented RPM
     Fusion `ffmpeg-devel`/`libva-devel` packages) actually produces a working binary end-to-end,
     given Moonlight-Qt's tagged release is ~19 months stale and `master` may have drifted from the
     README's exact dependency list.
   - Exact current `dnf copr enable lizardbyte/stable` package availability for the specific Fedora
     version CableDesk targets (the docs themselves warn this is version-timing-dependent).

4. **Discrepancy: Moonlight-Qt's release cadence vs. Sunshine's.** Sunshine ships near-weekly
   pre-releases and a stable roughly every 1–2 months; Moonlight-Qt's last tagged release (`v6.1.0`) is
   from 2024-09-17 — no new tag in ~19 months as of 2026-07-22, with only nightly CI builds continuing
   on `master`. This is worth flagging as a project-health/support asymmetry: CableDesk's Sunshine
   integration can track official stable tags, but its Moonlight-Qt integration likely needs to track
   `master` at a pinned commit rather than "the latest release," which has different reproducibility
   and update-cadence implications for CableDesk's own release process.

5. **`encoder = vulkan` and `capture = kwin`/portal-with-Vulkan are recent additions** (per the
   GamingOnLinux coverage of the May 2026 Sunshine release) and were not present in older Sunshine
   documentation versions found during search (e.g. `v0.21.0`/`v0.22.x` doc snapshots that predate this
   feature). If CableDesk pins to an older Sunshine release for stability, verify that release's
   `docs/configuration.md` actually lists `vulkan`/`kwin` as valid `capture`/`encoder` values before
   depending on them — this document's config-value tables reflect the **current `master`/latest-stable
   docs**, which may be ahead of whatever exact Sunshine version CableDesk ends up pinning.
