//! Notify systemd of service status changes

use std::{fmt, io, sync::Arc};

use camino::Utf8PathBuf;
use thiserror::Error;
use tokio::net::UnixDatagram;

use crate::socket::SocketError;

/// The environment variable that systemd uses to set the unix socket path
/// for notifications.
const NOTIFY_SOCKET: &str = "NOTIFY_SOCKET";

/// Error returned when sending a notification didn't work
#[derive(Debug, Error)]
pub enum NotifyError {
    /// An IO error occurred while sending the notification
    #[error("{}", .0)]
    IO(#[from] io::Error),

    /// A required environment variable was missing
    #[error("Missing ${0} variable")]
    MissingVar(&'static str),

    /// An environment variable had an invalid value
    #[error("Invalid ${0}={1}")]
    InvalidVar(&'static str, String),
}

impl From<SocketError> for NotifyError {
    fn from(value: SocketError) -> Self {
        match value {
            SocketError::IO(err) => NotifyError::IO(err),
            SocketError::MissingVar(var) => NotifyError::MissingVar(var),
            SocketError::InvalidVar(var, value) => NotifyError::InvalidVar(var, value),
            err => panic!("Unexpected error for Notify: {err}"),
        }
    }
}

/// Custom variable to send to SystemD
#[derive(Debug, Clone)]
pub struct CustomVariable {
    key: String,
    value: String,
}

impl fmt::Display for CustomVariable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "X-{}={}", self.key, self.value)
    }
}

/// Notification kinds to send to systemd
#[derive(Debug, Clone)]
pub enum Notification {
    /// Notify systemd that the service is ready
    Ready,

    /// Notify systemd that the service is reloading
    Reloading,

    /// Notify systemd that the service is stopping
    Stopping,

    /// Notify systemd of the service status
    Status(String),

    /// Notify systemd of an error number
    Errno(i32),

    /// Notify systemd that the service is ok (heartbeat)
    WatchdogOk,

    /// Notify systemd to trigger the watchdog
    WatchdogTrigger,

    /// Send a custom notification
    Custom(CustomVariable),
}

impl fmt::Display for Notification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Notification::Ready => f.write_str("READY=1"),
            Notification::Reloading => f.write_str("RELOADING=1"),
            Notification::Stopping => f.write_str("STOPPING=1"),
            Notification::Status(status) => write!(f, "STATUS={status}"),
            Notification::Errno(errno) => write!(f, "ERRNO={errno}"),
            Notification::WatchdogOk => f.write_str("WATCHDOG=1"),
            Notification::WatchdogTrigger => f.write_str("WATCHDOG=trigger"),
            Notification::Custom(variable) => write!(f, "{variable}"),
        }
    }
}

/// A systemd notification message, which
/// can consist of a series of known or custom systemd variables.
#[derive(Debug, Clone, Default)]
pub struct Message {
    variables: Vec<Notification>,
}

impl Message {
    /// Create a new message
    pub fn new() -> Self {
        Self {
            variables: Vec::new(),
        }
    }

    /// Add a notification to the message
    pub fn push(&mut self, notification: Notification) {
        self.variables.push(notification)
    }
}

impl From<Notification> for Message {
    fn from(value: Notification) -> Self {
        Message {
            variables: vec![value],
        }
    }
}

impl FromIterator<Notification> for Message {
    fn from_iter<I: IntoIterator<Item = Notification>>(iter: I) -> Self {
        let variables = iter.into_iter().collect();
        Message { variables }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for variable in &self.variables {
            writeln!(f, "{variable}")?;
        }
        Ok(())
    }
}

/// Notification socket for sending messages to Systemd
///
/// The default construction is to build this from the environment via [SystemDNotify::from_environment].
#[derive(Debug, Clone)]
pub struct SystemDNotify {
    socket: Arc<UnixDatagram>,
    address: Utf8PathBuf,
}

impl SystemDNotify {
    /// Create a new SystemDNotify client from the environment
    pub fn from_environment() -> Result<Self, NotifyError> {
        let address = crate::socket::var(NOTIFY_SOCKET)?.into();
        let socket = UnixDatagram::unbound()?;

        Ok(SystemDNotify {
            socket: Arc::new(socket),
            address,
        })
    }

    /// Send a message to systemd
    pub async fn send<M: Into<Message>>(&self, message: M) -> Result<(), NotifyError> {
        let message = message.into().to_string();
        self.socket
            .send_to(message.as_bytes(), &self.address)
            .await?;
        Ok(())
    }
}

/// Notify systemd that this service is ready.
///
/// This is implemented as sending a single message to systemd with the appropriate
/// ready command.
pub async fn ready() {
    if let Ok(notify) = SystemDNotify::from_environment() {
        if let Err(err) = notify.send(Notification::Ready).await {
            tracing::warn!("Failed to notify systemd: {err}");
        }
    }
}
