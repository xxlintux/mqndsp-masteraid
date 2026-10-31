Name:           masteraid
Version:        1.0.0
Release:        1%{?dist}
Summary:        Vintage Analog Mastering Chain plugin (CLAP and VST3)
License:        Proprietary
URL:            https://github.com/xxlintux/mqndsp-masteraid
BuildArch:      x86_64
Requires:       glibc >= 2.35, libX11, libxcb, libXcursor, mesa-libGL

%description
Masteraid is a mastering chain plugin featuring analog tape saturation,
mastering clipper, 4-band Massive Passive EQ with continuous proportional-Q,
vintage dynamics (Fairchild / Variable-Mu), true-peak limiter, and dual
precision metering.

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}%{_libdir}/clap
mkdir -p %{buildroot}%{_libdir}/vst3/Masteraid.vst3/Contents/x86_64-linux
mkdir -p %{buildroot}%{_docdir}/masteraid

# In Fedora x86_64, _libdir is /usr/lib64
cp %{_sourcedir}/Masteraid.clap %{buildroot}%{_libdir}/clap/
cp %{_sourcedir}/Masteraid.vst3/Contents/x86_64-linux/Masteraid.so \
   %{buildroot}%{_libdir}/vst3/Masteraid.vst3/Contents/x86_64-linux/

# Also provide symlinks in /usr/lib/clap and /usr/lib/vst3 for hosts scanning standard paths
mkdir -p %{buildroot}/usr/lib/clap
mkdir -p %{buildroot}/usr/lib/vst3
ln -sf %{_libdir}/clap/Masteraid.clap %{buildroot}/usr/lib/clap/Masteraid.clap
ln -sf %{_libdir}/vst3/Masteraid.vst3 %{buildroot}/usr/lib/vst3/Masteraid.vst3

if [ -f "%{_sourcedir}/README.txt" ]; then
    cp %{_sourcedir}/README.txt %{buildroot}%{_docdir}/masteraid/
fi
if [ -f "%{_sourcedir}/Masteraid_User_Manual.pdf" ]; then
    cp %{_sourcedir}/Masteraid_User_Manual.pdf %{buildroot}%{_docdir}/masteraid/
fi

%files
%{_libdir}/clap/Masteraid.clap
%{_libdir}/vst3/Masteraid.vst3
/usr/lib/clap/Masteraid.clap
/usr/lib/vst3/Masteraid.vst3
%doc %{_docdir}/masteraid/*

%changelog
* Sat Oct 31 2026 MQN DSP <contact@mqndsp.com> - 1.0.0-1
- Initial release of Masteraid for Linux (Fedora x86_64)
