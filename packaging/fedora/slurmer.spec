# Built on COPR through .copr/Makefile, which runs make-srpm.sh. That script
# sets Version from Cargo.toml and vendors every crate into Source1, so this
# build runs offline. Build locally with: packaging/fedora/make-srpm.sh <outdir>

# The release profile strips the binary, so there is no debug info to package.
%global debug_package %{nil}

Name:           slurmer
Version:        0.2.1
Release:        1%{?dist}
Summary:        Slurm job tracker over ssh

# slurmer itself is MIT. The binary statically links the vendored crates,
# whose licenses are listed in LICENSE.dependencies.
License:        MIT
URL:            https://github.com/shawonashraf/slurmer
Source0:        %{name}-%{version}.tar.gz
Source1:        %{name}-%{version}-vendor.tar.xz

ExclusiveArch:  x86_64 aarch64

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gcc
BuildRequires:  desktop-file-utils
BuildRequires:  pkgconfig(fontconfig)
BuildRequires:  pkgconfig(freetype2)
BuildRequires:  pkgconfig(vulkan)
BuildRequires:  pkgconfig(wayland-client)
BuildRequires:  pkgconfig(x11-xcb)
BuildRequires:  pkgconfig(xcb)
BuildRequires:  pkgconfig(xkbcommon)
BuildRequires:  pkgconfig(xkbcommon-x11)

# slurmer runs the system ssh; gpui loads Vulkan at runtime.
Requires:       openssh-clients
Requires:       vulkan-loader
Requires:       hicolor-icon-theme

%description
Slurmer is a desktop app that shows your Slurm job queue on one or more saved
HPC clusters. It runs squeue over the system ssh when you press Refresh, so
your ~/.ssh/config aliases, ProxyJump and ControlMaster settings apply.

%prep
%autosetup -n %{name}-%{version} -a 1
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF

%build
export RUSTFLAGS="%{?build_rustflags}"
cargo build --release --frozen --no-default-features -j %{_smp_build_ncpus}

%install
install -Dpm 0755 target/release/Slurmer %{buildroot}%{_bindir}/slurmer
desktop-file-install --dir=%{buildroot}%{_datadir}/applications \
    packaging/linux/org.shawonashraf.slurmer.desktop
for size in 32 64 128; do
    install -Dpm 0644 assets/icons/${size}x${size}.png \
        %{buildroot}%{_datadir}/icons/hicolor/${size}x${size}/apps/org.shawonashraf.slurmer.png
done
install -Dpm 0644 assets/icons/128x128@2x.png \
    %{buildroot}%{_datadir}/icons/hicolor/256x256/apps/org.shawonashraf.slurmer.png
install -Dpm 0644 assets/icons/icon.png \
    %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/org.shawonashraf.slurmer.png

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/org.shawonashraf.slurmer.desktop

%files
%license LICENSE LICENSE.dependencies
%doc README.md
%{_bindir}/slurmer
%{_datadir}/applications/org.shawonashraf.slurmer.desktop
%{_datadir}/icons/hicolor/*/apps/org.shawonashraf.slurmer.png

%changelog
* Thu Nov 05 2026 Shawon Ashraf <7804029+shawonashraf@users.noreply.github.com> - 0.2.1-1
- Draw a title bar with window buttons on GNOME Wayland
- Show the window title on Wayland

* Thu Nov 05 2026 Shawon Ashraf <7804029+shawonashraf@users.noreply.github.com> - 0.2.0-1
- First COPR package
