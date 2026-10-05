# SPDX-License-Identifier: MIT
# Source: https://github.com/hashdefault/compust/commit/3dfd920d3cffc0236d85331f9728ce6823231f8d

%global source_revision 3dfd920d3cffc0236d85331f9728ce6823231f8d
%global source_short 3dfd920

Name:           compust
Version:        0.3.0~beta.2+git20261005.3dfd920
Release:        0
Summary:        Experimental X11 compositor in Rust
License:        MIT
Group:          System/X11/Utilities
URL:            https://github.com/hashdefault/compust
# Named as Debian orig archives so the .dsc in this package uses the same files.
Source0:        %{name}_%{version}.orig.tar.gz
Source1:        %{name}_%{version}.orig-deps.tar.xz
BuildRequires:  cargo >= 1.95
BuildRequires:  fdupes
BuildRequires:  gcc
BuildRequires:  rust >= 1.95
BuildRequires:  xorg-x11-server-Xvfb
%if 0%{?fedora}
BuildRequires:  cargo-rpm-macros >= 26
BuildRequires:  mesa-dri-drivers
BuildRequires:  mesa-libEGL
Recommends:     mesa-libEGL
# The Fedora project configuration on OBS does not define rust_arches.
ExclusiveArch:  x86_64 aarch64
%else
BuildRequires:  cargo-packaging
BuildRequires:  Mesa-dri
BuildRequires:  Mesa-libEGL1
Recommends:     Mesa-libEGL1
ExclusiveArch:  %{rust_tier1_arches}
%endif

%description
Compust is an experimental X11 compositor written in Rust for Xorg and
XLibre. It provides fades, transparency, background blur, shadows,
rounded corners, per-window rules and configuration reloads. XRender is the
default renderer, with an optional OpenGL ES renderer and XRender fallback.

This is a source snapshot based on the 0.3.0-beta.2 prerelease;
BUILDINFO names its exact Git revision. Run it from an X11 session after
stopping the compositor already serving that screen.

%prep
%autosetup -n %{name}-%{source_short} -a1
%if 0%{?fedora}
# Fedora's macros write their own Cargo configuration for the vendored sources.
%cargo_prep -v vendor
%endif

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
    'upstream_package_version 0.3.0-beta.2' > BUILDINFO
rustc --version >> BUILDINFO
cargo --version >> BUILDINFO

# Install the notices here, not through %%license, so identical files can be linked.
install -Dm0644 LICENSE %{buildroot}%{_defaultlicensedir}/%{name}/LICENSE
find vendor -type f \( -iname '*license*' -o -iname 'copying*' -o -iname 'notice*' -o -iname 'copyright*' \) -print | \
    while IFS= read -r license; do
        install -Dm0644 "$license" "%{buildroot}%{_defaultlicensedir}/%{name}/dependency-licenses/${license#vendor/}"
    done
%fdupes %{buildroot}%{_defaultlicensedir}/%{name}

%files
%license %{_defaultlicensedir}/%{name}
%doc README.md README.pt-BR.md compust.example.toml BUILDINFO
%{_bindir}/compust

%changelog
