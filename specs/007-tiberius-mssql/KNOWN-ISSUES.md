# Tiberius macOS Integration: Known Issues & Fixes

**Date**: 2026-02-16
**Status**: Unmerged — branch `debug-windows-auth-failure` preserves working fixes
**Decision**: Deferred in favor of keeping odbc-api; revisit if zero-dependency
binary becomes a hard requirement.

## Summary

Three issues prevent tiberius 0.12.3 from working out-of-the-box on macOS with
Kerberos (Windows Integrated) auth. All three have known fixes applied on this
branch. The core TDS protocol implementation is solid once these are resolved.

## Issue 1: TLS handshake hangs on macOS (native-tls)

**Symptom**: `Client::connect()` hangs indefinitely after a successful TDS
prelogin exchange. The TLS ClientHello is never sent.

**Root cause**: Tiberius's `native-tls` feature uses Apple's Security.framework
on macOS. TDS wraps TLS *inside* TDS packets during the initial handshake
(via `TlsPreloginWrapper`), and Security.framework doesn't handle this correctly.

**Upstream issues**: prisma/tiberius#320, prisma/tiberius#364, prisma/tiberius#375

**Fix**: Switch from `native-tls` to `rustls` in Cargo.toml:
```toml
tiberius = { version = "0.12", default-features = false, features = ["tds73", "rustls", "sql-browser-tokio"] }
```

**Note**: Issue #363 documents that a tiberius maintainer recommends `rustls` on
non-Windows and `native-tls` on Windows. This is not reflected in the defaults
or documentation.

## Issue 2: GSSAPI null pointer crash (libgssapi 0.4.6)

**Symptom**: Panic in `libgssapi::util::Buf::deref()` with message:
`unsafe precondition(s) violated: slice::from_raw_parts requires the pointer
to be aligned and non-null`

**Root cause**: When GSSAPI returns an empty buffer `{length: 0, value: NULL}`,
libgssapi 0.4.6 passes the null pointer to `slice::from_raw_parts()`. Rust 1.78+
enforces the non-null precondition at runtime, turning silent UB into a crash.

**Upstream issue**: estokes/libgssapi#22 (fixed on main, never released as 0.4.x)

**Fix on tiberius main**: Commit `59db579` (Jul 2025) updates libgssapi to 0.8.1,
but no release has been published since v0.12.3 (Jul 2024).

**Workaround**: Point tiberius at git main instead of crates.io:
```toml
tiberius = { git = "https://github.com/prisma/tiberius", default-features = false, features = [...] }
```

**Minimal local fix** (for reference — apply to `libgssapi/src/util.rs` Buf::deref):
```rust
fn deref(&self) -> &Self::Target {
    if self.0.value.is_null() && self.0.length == 0 {
        return &[];
    }
    unsafe { slice::from_raw_parts(self.0.value.cast(), self.0.length as usize) }
}
```

## Issue 3: Kerberos SPN requires FQDN

**Symptom**: `GSSAPI Error: Server not found in Kerberos database` when using
a short hostname like `SVWTSTEM03`.

**Root cause**: Tiberius constructs the Kerberos SPN as
`MSSQLSvc/{host}:{port}` using whatever hostname the user provides
(`context.rs:62`). Active Directory registers SPNs with the FQDN
(e.g., `MSSQLSvc/svwtstem03.stjude.sjcrh.local:1433`). The short hostname
doesn't match. Microsoft's ODBC driver handles FQDN resolution internally.

**Fix**: Resolve the hostname to FQDN before passing to tiberius config.
See `resolve_fqdn()` in `src/backend/sqlserver.rs` on this branch. Uses the
`dns-lookup` crate for reverse DNS.

## Diagnostic methodology

The debugging session used a TCP proxy to capture and compare TDS prelogin
packets between ODBC (working) and tiberius (failing). Key technique:

1. Run a Python TCP proxy on localhost forwarding to the SQL Server
2. Point the client at the proxy
3. Log raw packet hex for comparison

This revealed that WARP/network was not the issue (both clients' prelogin
exchanges completed successfully through the proxy), isolating the problem
to tiberius's post-prelogin TLS initiation.

## Decision rationale

Tiberius is in maintenance mode (Prisma merges community PRs but doesn't
actively develop or release). The macOS + Kerberos path specifically is
under-tested. The ODBC driver is battle-tested and installable in all
target environments (developer workstations already have it; agent/CI
environments can install it via package manager or Dockerfile).

Revisit if:
- A concrete need arises for zero-dependency binaries in uncontrolled environments
- Tiberius publishes a new release with the libgssapi fix and rustls as default
- An alternative pure-Rust TDS client emerges
