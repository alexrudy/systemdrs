//! Systemd integration library
//!
//! This library provides a set of utilities for interacting with systemd from Rust.
//!
//! It eschews the use of libsystemd bindings in favor of using the `systemctl` command line utility
//! and environment variables to interact with systemd.

#[cfg(feature = "notify")]
pub mod notify;
pub mod properties;
pub mod socket;

pub use self::socket::sockets;
pub use self::socket::SystemDSocket;

/// Check if the current process is running under systemd as a service with the given unit name
pub fn is_systemd(unit: &str) -> bool {
    if let Ok(properties) = self::properties::properties(unit) {
        let systemd_pid = properties.property("MainPID");
        let process_pid = std::process::id().to_string();

        tracing::trace!(
            MainPID = ?systemd_pid,
            SelfPID = ?process_pid,
            "Systemd detected, checking for PID match"
        );

        return systemd_pid == Some(&process_pid);
    }

    // If we can't read the properties, we're not running under systemd
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_systemd() {
        assert!(
            !is_systemd("automoton.service"),
            "Tests should not be running under the automoton.service systemd unit"
        );
    }
}
