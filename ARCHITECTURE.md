\# Below is the immutable directory structure for this repository. 

\# 1. DO NOT suggest modifications, deletions, or renames to the packaging environments or .GitHub workflows

\# 2. All new business logic must remain strictly confined to the src directory.

\# 3. When referencing paths or generating commands, strictly utilize the exact naming conventions and relative paths mapped below. Do not hallucinate parent directories.



<pre>

./                                                    # Root directory (Current project folder)

├── .github/                                          # GitHub configuration folder

│   └── workflows/                                    # GitHub Actions CI/CD automation pipelines

│       ├── build-linux-binary.yml                    # Workflow to compile the raw Linux ELF binary

│       ├── build-windows-binary.yml                  # Workflow to compile the raw Windows .exe binary

│       ├── ci.yml                                    # Workflow for daily code linting and unit testing

│       ├── build-package-apk.yml                     # Dispatcher running packaging/apk/build-alpine-apk.sh

│       ├── build-package-appimage.yml                # Dispatcher running packaging/appimage/build-appimage.sh

│       ├── build-package-aur.yml                     # Dispatcher running packaging/aur/build-arch-aur.sh

│       ├── build-package-deb.yml                     # Dispatcher running packaging/deb/build-debian-package.sh

│       ├── build-package-flatpak.yml                 # Dispatcher running packaging/flatpak/build-flatpak.sh

│       ├── build-package-msi.yml                     # Dispatcher running packaging/wix/build-windows-msi.ps1

│       ├── build-package-nix.yml                     # Dispatcher running packaging/nix/build-nix-flake.sh

│       ├── build-package-rpm.yml                     # Dispatcher running packaging/rpm/build-redhat-rpm.sh

│       └── build-package-winget.yml                  # Dispatcher running packaging/winget/publish-to-winget.ps1

├── assets/                                           # Directory for static visual assets

│   └── brand/                                        # Brand-specific logos and imagery

│       ├── app.ico                                   # Icon file for Windows executables/shortcuts

│       └── app\_icon.png                              # Standard high-res icon for Linux desktops

├── dist/                                             # Local build output directory

│   ├── binaries/                                     # Folder for raw, un-packaged compiled executables

│   │   ├── rwif                                 # Compiled raw binary for Linux

│   │   └── rwif.exe                             # Compiled raw binary for Windows

│   └── packages/                                     # Folder for final bundled distribution formats

│       ├── rwif.apk                             # Compiled Alpine Linux package

│       ├── rwif.appimage                        # Compiled universal Linux portable executable

│       ├── rwif.deb                             # Compiled Debian/Ubuntu installation package

│       ├── rwif.msi                             # Compiled Windows installer package

│       └── rwif.rpm                             # Compiled RedHat/Fedora installation package

├── docs/                                             # Deep-dive documentation for users/contributors

│   └── CONFIGURATION.md                              # Guide on how to configure the app (env vars, config)

├── packaging/                                        # Isolated build environments for distribution formats

│   ├── apk/                                          # Alpine Linux package environment

│   │   ├── APKBUILD                                  # Build recipe used by Alpine's 'abuild' tool

│   │   └── build-alpine-apk.sh                       # Isolated script to execute the APK build process

│   ├── appimage/                                     # AppImage universal Linux environment

│   │   ├── AppRun                                    # Entrypoint script executed by the AppImage

│   │   ├── appimage-builder.yml                      # Configuration for the AppImage builder tool

│   │   └── build-appimage.sh                         # Isolated script to bundle the AppImage

│   ├── aur/                                          # Arch User Repository environment

│   │   ├── PKGBUILD                                  # Build recipe used by Arch's 'makepkg' tool

│   │   └── build-arch-aur.sh                         # Isolated script to test the AUR build locally

│   ├── completions/                                  # Shell auto-completion scripts for CLI usability

│   │   ├── generate-completions.sh                   # Script to auto-generate below files via Rust 'clap'

│   │   ├── rwif.bash                            # Auto-completion logic for Bash shell

│   │   ├── rwif.fish                            # Auto-completion logic for Fish shell

│   │   ├── rwif.nu                              # Auto-completion logic for Nushell

│   │   ├── rwif.ps1                             # Auto-completion logic for PowerShell

│   │   └── rwif.zsh                             # Auto-completion logic for Zsh shell

│   ├── deb/                                          # Debian/Ubuntu package environment

│   │   ├── build-debian-package.sh                   # Isolated script to stage files and run 'dpkg-deb'

│   │   └── debian/                                   # Staging directory for dpkg metadata

│   │       ├── control                               # Package metadata (dependencies, architecture)

│   │       ├── postinst                              # Script executed AFTER package installs

│   │       └── prerm                                 # Script executed BEFORE package uninstalls

│   ├── desktop/                                      # Standard Linux desktop integration files

│   │   ├── rwif.1                               # Linux man page documentation for terminal users

│   │   └── rwif.desktop                         # Linux application launcher shortcut and metadata

│   ├── flatpak/                                      # Flatpak sandboxed application environment

│   │   ├── build-flatpak.sh                          # Isolated script to execute 'flatpak-builder'

│   │   └── org.local76.rwif.yaml                # Flatpak manifest defining dependencies

│   ├── nix/                                          # NixOS / Nix package manager environment

│   │   ├── build-nix-flake.sh                        # Isolated script to execute the Nix build

│   │   └── default.nix                               # Nix expression defining how to build the application

│   ├── rpm/                                          # RedHat/Fedora package environment

│   │   ├── build-redhat-rpm.sh                       # Isolated script to set up rpmbuild tree and execute

│   │   └── rwif.spec                            # RPM specification file (Name, Version, %prep, %build)

│   ├── winget/                                       # Windows Package Manager environment

│   │   ├── publish-to-winget.ps1                     # PowerShell script to submit YAML via wingetcreate

│   │   └── winget.yaml                               # Winget manifest defining the installer URL and hashes

│   └── wix/                                          # Windows Installer XML (WiX) environment

│       ├── build-windows-msi.ps1                     # PowerShell script to run WiX tools

│       └── main.wxs                                  # XML definition of the Windows installer UI and payload

├── src/                                              # Core Rust source code directory

│   ├── config.rs                                     # Module handling configuration parsing

│   ├── input.rs                                      # Module handling CLI arguments or standard input

│   ├── logger.rs                                     # Module handling terminal output and logging

│   ├── main.rs                                       # Primary application entry point

│   └── worker.rs                                     # Module containing the core business logic

├── tests/                                            # Top-level directory for Rust integration testing

│   └── integration\\\_test.rs                           # Tests executing the compiled binary from the outside

├── web/                                              # Directory for web-related assets or local dashboards

│   └── index.html                                    # HTML file for local UI or basic documentation

├── .gitignore                                        # Tells Git to ignore specific files

├── build.rs                                          # Rust build script executed before compilation

├── Cargo.lock                                        # Pinned exact versions of dependencies (do not edit)

├── Cargo.toml                                        # Rust package manifest (name, version, dependencies)

├── ARCHITECTURE.md                                   # CRITICAL ARCHITECTURE DIRECTIVE FOR AI AGENTS \& LLMs

├── CHANGELOG.md                                      # Sequential record of features and bug fixes

├── CONTRIBUTING.md                                   # Guidelines for external developers

├── COPYRIGHT.md                                      # Detailed copyright attributions and third-party notices

├── LICENSE.md                                        # The legal open-source license governing this repo

├── PRIVACY.md                                        # Explanation of telemetry, data usage, and user privacy

├── README.md                                         # The storefront file: Introduction, install guide

├── SECURITY.md                                       # Instructions for reporting security vulnerabilities

└── SUPPORT.md                                        # Where users can go to get help (Discord, Issues, Emails)

</pre>



