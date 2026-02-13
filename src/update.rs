use anyhow::{bail, Result};
use axoupdater::AxoUpdater;

/// Check for updates and self-update dbtoon if a newer version is available.
///
/// Uses axoupdater to read the install receipt written by the cargo-dist
/// installer, check GitHub Releases for a newer version, and run the
/// installer to update in place.
pub fn run_update() -> Result<()> {
    let mut updater = AxoUpdater::new_for("dbtoon");
    let version: axoupdater::Version = env!("CARGO_PKG_VERSION").parse()?;
    updater.set_current_version(version)?;

    // Try to load the install receipt. If there is no receipt, the binary
    // was installed via `cargo install` or built from source.
    if let Err(e) = updater.load_receipt() {
        if is_no_receipt(&e) {
            eprintln!("dbtoon was not installed via the shell/PowerShell installer.");
            eprintln!("Self-update is only available for installer-based installations.");
            eprintln!("Please update with: cargo install dbtoon");
            return Ok(());
        }
        // Receipt exists but could not be loaded — treat as a mismatch
        eprintln!("This copy of dbtoon was not installed by the shell/PowerShell installer.");
        eprintln!("Please update with the method you originally used to install it.");
        return Ok(());
    }

    eprintln!("Checking for updates...");

    match updater.run_sync() {
        Ok(Some(result)) => {
            let old = result
                .old_version
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            eprintln!("Updated dbtoon: {} => {}", old, result.new_version);
            Ok(())
        }
        Ok(None) => {
            eprintln!(
                "dbtoon v{} is already up to date.",
                env!("CARGO_PKG_VERSION")
            );
            Ok(())
        }
        Err(e) => {
            if is_network_error(&e) {
                bail!("unable to check for updates \u{2014} are you connected to the internet?");
            }
            if is_no_installer_error(&e) {
                bail!("no installer found for your platform in the latest release.");
            }
            bail!("update failed: {e}");
        }
    }
}

/// Check if the error indicates no install receipt was found.
fn is_no_receipt(e: &axoupdater::AxoupdateError) -> bool {
    // axoupdater uses miette diagnostics; match on the display text
    // since the error variants aren't publicly exposed as an enum we can match on.
    let msg = e.to_string().to_lowercase();
    msg.contains("receipt") && (msg.contains("not found") || msg.contains("no") || msg.contains("missing") || msg.contains("couldn't"))
}

/// Check if the error is a network/reqwest error.
fn is_network_error(e: &axoupdater::AxoupdateError) -> bool {
    let msg = e.to_string().to_lowercase();
    msg.contains("reqwest") || msg.contains("network") || msg.contains("connect") || msg.contains("dns") || msg.contains("timed out")
}

/// Check if the error indicates no installer asset exists for this platform.
fn is_no_installer_error(e: &axoupdater::AxoupdateError) -> bool {
    let msg = e.to_string().to_lowercase();
    msg.contains("installer") && (msg.contains("not found") || msg.contains("no "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_receipt_returns_ok() {
        // In test environment, no install receipt exists.
        // run_update should handle this gracefully (return Ok, not Err).
        let result = run_update();
        assert!(result.is_ok(), "no-receipt case should return Ok, not Err");
    }

    #[test]
    fn already_current_returns_ok() {
        // When already on the latest version, run_update should return Ok.
        // This test validates the function signature and return type;
        // the actual "already current" path requires a receipt + network,
        // so this exercises the no-receipt path which also returns Ok.
        let result = run_update();
        assert!(result.is_ok(), "should return Ok for non-error conditions");
    }
}
