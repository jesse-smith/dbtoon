# Feature Specification: Cross-Platform Binary Distribution & Self-Update

**Feature Branch**: `006-cargo-dist-release`
**Created**: 2026-02-12
**Status**: Draft
**Input**: User description: "Create CI workflow for cargo-dist building targets for 64 bit Windows, Linux, and Mac (Apple and Intel silicon). Use canonical Github actions where possible. Also add dbtoon update command for built-in self updating. Update readme with install and update instructions."

## Clarifications

### Session 2026-02-12

- Q: Is the GitHub repository public or private? → A: Public repo — installers and update command work without authentication.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Install dbtoon from a release (Priority: P1)

A team member who does not have Rust installed needs to get dbtoon running on their machine. They visit the project's GitHub repository (or receive a shared link) and run a one-liner install command appropriate to their platform. The tool is downloaded, placed on their PATH, and ready to use immediately.

**Why this priority**: Without installation, no other feature matters. This is the fundamental enabler for team adoption.

**Independent Test**: Can be fully tested by running the platform-appropriate install command on a clean machine and then executing `dbtoon --version` to confirm it works.

**Acceptance Scenarios**:

1. **Given** a macOS (Apple Silicon) machine without Rust installed, **When** the user runs the shell installer command, **Then** dbtoon is installed to a standard location on their PATH and `dbtoon --version` prints the installed version.
2. **Given** a macOS (Intel) machine without Rust installed, **When** the user runs the shell installer command, **Then** dbtoon is installed and functional.
3. **Given** a Windows 64-bit machine, **When** the user runs the PowerShell installer command, **Then** dbtoon is installed and `dbtoon --version` works from a new terminal session.
4. **Given** a Linux/WSL (x86_64) machine, **When** the user runs the shell installer command, **Then** dbtoon is installed and functional.

---

### User Story 2 - Automated release pipeline (Priority: P1)

A project maintainer pushes a version tag to trigger an automated build. The CI pipeline compiles platform-specific binaries for all supported targets and publishes them as a GitHub Release with installer scripts, requiring no manual build or upload steps.

**Why this priority**: Tied with installation — without automated builds, there are no binaries to install. This is the supply side of the distribution story.

**Independent Test**: Can be tested by pushing a version tag to the repository and verifying that a GitHub Release appears with binaries for all four targets and installer scripts attached.

**Acceptance Scenarios**:

1. **Given** the repository has the release workflow configured, **When** a maintainer pushes a tag matching the version pattern (e.g., `v0.2.0`), **Then** the CI pipeline builds binaries for all four supported targets.
2. **Given** the CI pipeline completes successfully, **When** the release is published, **Then** the GitHub Release contains downloadable archives for each target plus shell and PowerShell installer scripts.
3. **Given** the CI pipeline encounters a build failure on any target, **When** the workflow completes, **Then** the release is not published and the failure is clearly reported.

---

### User Story 3 - Self-update installed binary (Priority: P2)

A team member who already has dbtoon installed wants to update to the latest version. They run `dbtoon update` and the tool checks for a newer release, downloads it, and replaces itself — no need to re-run the original install script or visit GitHub.

**Why this priority**: Lowers the friction of staying current. Without this, users must remember the install command or navigate to GitHub manually for each update.

**Independent Test**: Can be tested by installing an older version, running `dbtoon update`, and verifying that `dbtoon --version` reflects the newer version.

**Acceptance Scenarios**:

1. **Given** an older version of dbtoon is installed, **When** the user runs `dbtoon update`, **Then** the tool downloads and installs the latest release and reports the new version.
2. **Given** the latest version is already installed, **When** the user runs `dbtoon update`, **Then** the tool reports that no update is available and exits cleanly.
3. **Given** the machine has no internet connectivity, **When** the user runs `dbtoon update`, **Then** the tool reports a clear error message indicating it cannot check for updates.

---

### User Story 4 - Find install and update instructions (Priority: P3)

A team member reads the project README to learn how to install dbtoon or update an existing installation. The README contains clear, copy-pasteable commands for each supported platform.

**Why this priority**: Documentation makes the install and update flows discoverable. Lower priority because the installer scripts themselves contain usage hints, but important for onboarding.

**Independent Test**: Can be tested by reading the README and following the documented commands on each platform.

**Acceptance Scenarios**:

1. **Given** a user visits the project README, **When** they look for installation instructions, **Then** they find platform-specific install commands for macOS, Windows, and Linux/WSL.
2. **Given** a user has dbtoon installed, **When** they look for update instructions in the README, **Then** they find documentation for the `dbtoon update` command.

---

### Edge Cases

- What happens when a user runs `dbtoon update` but the binary was installed via `cargo install` rather than from a release? The update command should detect this gracefully and advise the user to use `cargo install` to update instead.
- What happens when a user lacks write permissions to the binary's install location? The update command should report a clear permissions error.
- What happens if a release is in progress (partially uploaded) when a user runs `dbtoon update`? The tool should validate the download before replacing the existing binary.
- What happens on a platform for which no binary is published? The installer script should fail with a clear "unsupported platform" message.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The project MUST produce pre-built binaries for four targets: 64-bit Windows, 64-bit Linux, macOS Apple Silicon, and macOS Intel.
- **FR-002**: The project MUST automatically build and publish binaries to GitHub Releases when a version tag is pushed.
- **FR-003**: Each GitHub Release MUST include platform-appropriate installer scripts (shell script for macOS/Linux, PowerShell script for Windows).
- **FR-004**: The installer scripts MUST place the binary on the user's PATH without requiring manual configuration.
- **FR-005**: The tool MUST provide an `update` subcommand that checks for and installs newer versions from GitHub Releases.
- **FR-006**: The `update` subcommand MUST report the current version, the available version, and whether an update was performed.
- **FR-007**: The `update` subcommand MUST NOT downgrade the installed version (it should only offer the latest stable release).
- **FR-008**: The project README MUST include installation instructions for all supported platforms.
- **FR-009**: The project README MUST document the `update` subcommand.
- **FR-010**: The release workflow MUST NOT interfere with the existing CI workflow (tests, clippy, coverage).

### Key Entities

- **Release**: A versioned set of platform-specific binaries and installer scripts published to GitHub.
- **Target Platform**: A combination of OS and architecture for which a binary is built (e.g., 64-bit Windows, macOS Apple Silicon).
- **Installed Binary**: The dbtoon executable on a user's machine, which knows its own version and can check for updates.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A team member can install dbtoon on any supported platform in under 60 seconds using a single command, without needing Rust or any build tools.
- **SC-002**: A maintainer can publish a new release to all four platforms by pushing a single version tag — no manual build or upload steps required.
- **SC-003**: A team member can update their installed copy to the latest version by running `dbtoon update` in under 30 seconds.
- **SC-004**: All four target platforms (Windows x64, Linux x64, macOS ARM64, macOS x64) have downloadable binaries in every release.

## Assumptions

- The GitHub repository is public. Installer scripts and the update command use unauthenticated HTTPS downloads from GitHub Releases.
- Team members have internet access to download releases and check for updates.
- The version tag format follows semantic versioning with a `v` prefix (e.g., `v0.2.0`), which is the cargo-dist convention.
- Only stable releases are published — no pre-release or nightly builds are in scope.
- The existing `odbc-api` dependency requires ODBC drivers on the target machine at runtime; this is a prerequisite for SQL Server functionality regardless of how dbtoon is installed, and is outside the scope of this feature.
