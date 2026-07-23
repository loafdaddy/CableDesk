# Hardware Test Plan

**Status:** Deferred — not yet physically tested.

CableDesk has **not** been validated with real USB4/Thunderbolt hardware or a
second machine. Items below are the future checklist. Do not mark them failed;
do not fabricate results.

## Controllers and cable

- [ ] Confirm USB4 or Thunderbolt controllers on both computers
- [ ] Confirm cable compatibility (USB4/Thunderbolt data, not charge-only)
- [ ] Confirm ordinary USB-C without USB4 is rejected honestly
- [ ] Confirm `thunderbolt-net` / USB4NET module loads
- [ ] Confirm direct interface creation
- [ ] Record actual interface names and sysfs parent paths

## Networking

- [ ] Confirm NetworkManager CableDesk profile activation
- [ ] Confirm link-local address assignment
- [ ] Confirm direct ping between peers
- [ ] Confirm direct-interface-scoped mDNS
- [ ] Confirm Wi-Fi is not used for discovery or streaming
- [ ] Confirm ordinary Ethernet is not used
- [ ] Confirm no default route / DNS / NAT from the CableDesk profile
- [ ] Confirm cable removal ends the session immediately
- [ ] Confirm reconnection after re-plug

## Sunshine / Moonlight

- [ ] Confirm managed Sunshine is not exposed on Wi-Fi/LAN
- [ ] Confirm Moonlight uses only the validated direct endpoint
- [ ] Confirm desktop video, keyboard, mouse, audio
- [ ] Measure encode / link / decode / render latency

## Power

- [ ] Confirm charging / not charging / discharging reporting
- [ ] Confirm charge rate where exposed
- [ ] Confirm battery behaviour under stream load

## Security

- [ ] Confirm untrusted peer cannot stream
- [ ] Confirm trusted peer on Wi-Fi/LAN is rejected
- [ ] Confirm firewalld zone binding on the direct iface only
