Name:           rwif
Version:        TEMPLATE_VERSION
Release:        1%{?dist}
Summary:        A project template for creating unified local terminal utilities in Rust
License:        MIT
URL:            https://github.com/local76/rWifi
Source0:        %{name}-%{version}.tar.gz

%description
A project template for creating unified local terminal utilities in Rust.

%prep
%setup -q

%build
cargo build --release --locked

%install
rm -rf $RPM_BUILD_ROOT
install -d $RPM_BUILD_ROOT/%{_bindir}
install -m 755 target/release/rwif $RPM_BUILD_ROOT/%{_bindir}/rwif

%files
%{_bindir}/rwif
