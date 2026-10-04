# SPDX-License-Identifier: MIT
# Source snapshot: https://github.com/hashdefault/compust/commit/681eae243cbf31d723928ce39317acd92a26243a

%global source_revision 681eae243cbf31d723928ce39317acd92a26243a
%global source_short 681eae2

Name:           compust
Version:        0.3.0~beta.1+git20261004.681eae2
Release:        0
Summary:        Experimental X11 compositor in Rust
License:        MIT
Group:          System/X11/Utilities
URL:            https://github.com/hashdefault/compust
Source0:        %{name}-%{source_short}.tar.gz
Source1:        %{name}-vendor-%{source_short}.tar.zst
BuildRequires:  cargo >= 1.95
BuildRequires:  cargo-packaging
BuildRequires:  gcc
BuildRequires:  Mesa-dri
BuildRequires:  Mesa-libEGL1
BuildRequires:  rust >= 1.95
BuildRequires:  xorg-x11-server-Xvfb
BuildRequires:  zstd
Recommends:     Mesa-libEGL1
ExclusiveArch:  %{rust_tier1_arches}

%description
Compust is an experimental X11 compositor written in Rust for Xorg and
XLibre. It provides fades, transparency, background blur, rectangular
shadows, per-window rules and configuration reloads. XRender is the
default renderer, with an optional OpenGL ES renderer and XRender fallback.

This package is a snapshot of main after 0.3.0-beta.1, identified by its
Git revision in the package version and BUILDINFO. Run it from an X11
session after stopping the compositor already serving that screen.

%prep
%autosetup -n %{name}-%{source_short} -a1

%build
%{cargo_build} --locked --bin compust

%check
# Prepare the directory once so concurrent Xvfb servers do not race to create it.
if [ ! -d /tmp/.X11-unix ]; then
    install -d -m1777 /tmp/.X11-unix
fi
export COMPUST_GPU_TESTS=1
export LIBGL_ALWAYS_SOFTWARE=1
%{cargo_test} --locked --workspace --all-targets
target/release/compust --check-config --config compust.example.toml

%install
install -Dm0755 target/release/compust %{buildroot}%{_bindir}/compust
printf '%s\n' \
    'upstream_revision %{source_revision}' \
    'snapshot_version %{version}' \
    'upstream_package_version 0.3.0-beta.1' > BUILDINFO
rustc --version >> BUILDINFO
cargo --version >> BUILDINFO

mkdir dependency-licenses
find vendor -type f \( -iname '*license*' -o -iname 'copying*' -o -iname 'notice*' -o -iname 'copyright*' \) -print | \
    while IFS= read -r license; do
        install -Dm0644 "$license" "dependency-licenses/${license#vendor/}"
    done

%files
%license LICENSE dependency-licenses
%doc README.md README.pt-BR.md compust.example.toml BUILDINFO
%{_bindir}/compust

%changelog
