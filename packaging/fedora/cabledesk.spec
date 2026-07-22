%global cargo_workspace_name cabledesk

Name:           cabledesk
Version:        0.1.0
Release:        1%{?dist}
Summary:        Ultra-low-latency wired desktop connection over USB4/Thunderbolt

# CableDesk itself is GPL-3.0-or-later. Vendored dependencies keep their
# own (compatible, permissive-or-copyleft) licenses; a full third-party
# license summary must be generated with %%cargo_license_summary (or
# cargo-license) once real releases are cut. See docs/PACKAGING.md.
License:        GPL-3.0-or-later
URL:            https://github.com/cabledesk/cabledesk
Source0:        %{name}-%{version}.tar.zst
# Regenerate with `cargo vendor` before every release; see
# scripts/build-rpm.sh. Not committed to git (large, regenerable).
Source1:        %{name}-%{version}-vendor.tar.zst

BuildRequires:  rust-packaging >= 25
BuildRequires:  gcc
BuildRequires:  pkgconfig(gtk4) >= 4.14
BuildRequires:  pkgconfig(libadwaita-1) >= 1.4
BuildRequires:  pkgconfig(dbus-1)
BuildRequires:  desktop-file-utils
BuildRequires:  libappstream-glib
BuildRequires:  systemd-rpm-macros

# This is a workspace of three binaries sharing internal library crates
# (see docs/ARCHITECTURE.md); rust2rpm's per-binary %%files split below
# follows the "one source package, multiple binary subpackages" model
# documented in docs/PACKAGING.md section 1.4, not the "one Fedora package
# per published crate" model (which applies to standalone library crates,
# not to this application's own internal workspace members).

%description
CableDesk connects two Linux computers over a direct USB4 or Thunderbolt
cable and provides an ultra-low-latency, near-local desktop-control
experience. It is a native GTK4/libadwaita automation layer around Sunshine
and Moonlight, not a replacement for either.

This is an experimental project. An ordinary USB-C cable and port do not
guarantee compatibility: both computers need genuine USB4 or Thunderbolt
support. See the project README for current limitations — pairing,
networking and streaming are not implemented yet in this release.

%package agent
Summary:        CableDesk user-session agent
Requires:       %{name} = %{version}-%{release}
%description agent
The unprivileged, per-user CableDesk background service
(cabledesk-agent.service, a systemd --user unit). Maintains device
identity, discovery and (in later releases) pairing and connection state.

%package helper
Summary:        CableDesk privileged system helper
Requires:       %{name} = %{version}-%{release}
Requires:       polkit
Requires:       NetworkManager
Requires:       firewalld
%description helper
The privileged, narrowly-scoped CableDesk system service
(cabledesk-helper.service). Every privileged action it exposes is
authorised individually via Polkit — see docs/SECURITY.md. This release
implements no privileged actions yet (see crates/cabledesk-helper).

%package selinux
Summary:        CableDesk SELinux policy
BuildArch:      noarch
Requires:       %{name}-helper = %{version}-%{release}
%description selinux
Placeholder subpackage for the future CableDesk SELinux policy module. No
policy module is shipped yet — see data/selinux/README.md for the plan.
This subpackage currently installs nothing; it exists so the dependency
shape (an optional, separately-installable SELinux add-on) is correct from
the start.

%prep
%autosetup -n %{name}-%{version} -p1
%cargo_prep -V1 -v %{SOURCE1}

%generate_buildrequires
%cargo_generate_buildrequires -a

%build
%cargo_build -a

%install
%cargo_install -a --bin cabledesk
%cargo_install -a --bin cabledesk-agent
%cargo_install -a --bin cabledesk-helper
%cargo_install -a --bin cabledeskctl

# cabledesk-helper is a privileged system binary; conventionally shipped
# under %{_libexecdir}, not %{_bindir}, since it is not meant to be run
# directly by users.
mkdir -p %{buildroot}%{_libexecdir}
mv %{buildroot}%{_bindir}/cabledesk-helper %{buildroot}%{_libexecdir}/cabledesk-helper

install -Dm0644 data/applications/org.cabledesk.CableDesk.desktop \
  %{buildroot}%{_datadir}/applications/org.cabledesk.CableDesk.desktop
install -Dm0644 data/metainfo/org.cabledesk.CableDesk.metainfo.xml \
  %{buildroot}%{_metainfodir}/org.cabledesk.CableDesk.metainfo.xml
install -Dm0644 data/icons/hicolor/scalable/apps/org.cabledesk.CableDesk.svg \
  %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/org.cabledesk.CableDesk.svg

install -Dm0644 data/systemd/user/cabledesk-agent.service \
  %{buildroot}%{_userunitdir}/cabledesk-agent.service
install -Dm0644 data/systemd/system/cabledesk-helper.service \
  %{buildroot}%{_unitdir}/cabledesk-helper.service

install -Dm0644 data/dbus-1/interfaces/org.cabledesk.Agent1.xml \
  %{buildroot}%{_datadir}/dbus-1/interfaces/org.cabledesk.Agent1.xml
install -Dm0644 data/dbus-1/interfaces/org.cabledesk.Helper1.xml \
  %{buildroot}%{_datadir}/dbus-1/interfaces/org.cabledesk.Helper1.xml
install -Dm0644 data/dbus-1/system.d/org.cabledesk.Helper1.conf \
  %{buildroot}%{_datadir}/dbus-1/system.d/org.cabledesk.Helper1.conf

install -Dm0644 data/polkit-1/actions/org.cabledesk.Helper1.policy \
  %{buildroot}%{_datadir}/polkit-1/actions/org.cabledesk.Helper1.policy

install -Dm0644 data/firewalld/zones/cabledesk.xml \
  %{buildroot}%{_prefix}/lib/firewalld/zones/cabledesk.xml

install -Dm0644 data/modules-load.d/cabledesk.conf \
  %{buildroot}%{_prefix}/lib/modules-load.d/cabledesk.conf

install -Dm0644 data/udev/rules.d/70-cabledesk.rules \
  %{buildroot}%{_udevrulesdir}/70-cabledesk.rules

%check
%cargo_test -a

desktop-file-validate %{buildroot}%{_datadir}/applications/org.cabledesk.CableDesk.desktop
appstream-util validate-relax --nonet \
  %{buildroot}%{_metainfodir}/org.cabledesk.CableDesk.metainfo.xml

%post agent
%systemd_user_post cabledesk-agent.service

%preun agent
%systemd_user_preun cabledesk-agent.service

%post helper
%systemd_post cabledesk-helper.service

%preun helper
%systemd_preun cabledesk-helper.service

%postun helper
%systemd_postun_with_restart cabledesk-helper.service

%files
%license LICENSE
%doc README.md
%{_bindir}/cabledesk
%{_bindir}/cabledeskctl
%{_datadir}/applications/org.cabledesk.CableDesk.desktop
%{_metainfodir}/org.cabledesk.CableDesk.metainfo.xml
%{_datadir}/icons/hicolor/scalable/apps/org.cabledesk.CableDesk.svg

%files agent
%{_bindir}/cabledesk-agent
%{_userunitdir}/cabledesk-agent.service
%{_datadir}/dbus-1/interfaces/org.cabledesk.Agent1.xml

%files helper
%{_libexecdir}/cabledesk-helper
%{_unitdir}/cabledesk-helper.service
%{_datadir}/dbus-1/interfaces/org.cabledesk.Helper1.xml
%{_datadir}/dbus-1/system.d/org.cabledesk.Helper1.conf
%{_datadir}/polkit-1/actions/org.cabledesk.Helper1.policy
%{_prefix}/lib/firewalld/zones/cabledesk.xml
%{_prefix}/lib/modules-load.d/cabledesk.conf
%{_udevrulesdir}/70-cabledesk.rules

%files selinux
# Intentionally empty — see data/selinux/README.md.

%changelog
* Wed Jul 22 2026 CableDesk contributors <cabledesk@example.invalid> - 0.1.0-1
- Initial experimental packaging: read-only compatibility detection only.
  No pairing, networking or streaming yet. See docs/ROADMAP.md.

# Whether CableDesk will ultimately build through Fedora's dist-git+koji
# pipeline (where %%autorelease/%%autochangelog are the current default
# recommendation) or ship as a standalone COPR/self-hosted project is not
# yet decided (see docs/OPEN_QUESTIONS.md) — a hand-written changelog is
# the safe default until that's settled, since %%autochangelog requires
# the rpmautospec dist-git tooling to function.
