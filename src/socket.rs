//! Access sockets passed from systemd

use std::fs::File;
use std::io;
use std::net::TcpListener;
use std::os::unix::prelude::*;
use std::process;

use thiserror::Error;

const SD_FD_OFFSET: i32 = 3;
const LISTEN_FDS: &str = "LISTEN_FDS";
const LISTEN_FDNAMES: &str = "LISTEN_FDNAMES";
const LISTEN_PID: &str = "LISTEN_PID";

/// Errors that can occur when trying to access systemd-owned sockets
#[derive(Debug, Error)]
pub enum SocketError {
    /// An IO error occurred communicating with the socket
    #[error("{}", .0)]
    IO(#[from] io::Error),

    /// The PID that systemd gave us is not our PID
    #[error("PID={0} but ${}={1}", LISTEN_PID)]
    WrongPID(u32, String),

    /// The file descriptor that systemd gave us is not a socket
    #[error("file descriptor {} is not a socket", .0)]
    NotSocket(RawFd),

    /// Missing a systemd variable
    #[error("Missing ${0} variable")]
    MissingVar(&'static str),

    /// Invalid value for a systemd variable
    #[error("Invalid ${0}={1}")]
    InvalidVar(&'static str, String),
}

pub(crate) fn var(name: &'static str) -> Result<String, SocketError> {
    match std::env::var(name) {
        Ok(value) => Ok(value),
        Err(std::env::VarError::NotPresent) => Err(SocketError::MissingVar(name)),
        Err(std::env::VarError::NotUnicode(_)) => Err(SocketError::MissingVar(name)),
    }
}

/// Get the sockets that systemd has passed to us as file descriptors
pub fn sockets() -> Result<Vec<SystemDSocket>, SocketError> {
    let listen_pid = var(LISTEN_PID);
    let listen_fds = var(LISTEN_FDS);
    let listen_fd_names = var(LISTEN_FDNAMES).ok();

    construct_sockets(
        listen_fds?.as_str(),
        listen_fd_names.as_deref(),
        listen_pid?.as_str(),
    )
}

fn construct_sockets(
    listen_fds: &str,
    listen_fd_names: Option<&str>,
    listen_pid: &str,
) -> Result<Vec<SystemDSocket>, SocketError> {
    let pid = listen_pid
        .parse::<u32>()
        .map_err(|_| SocketError::InvalidVar(LISTEN_PID, listen_pid.into()))?;

    if process::id() != pid {
        return Err(SocketError::WrongPID(process::id(), listen_pid.into()));
    }

    let n = listen_fds
        .parse::<usize>()
        .map_err(|_| SocketError::InvalidVar(LISTEN_FDS, listen_fds.into()))?;

    if let Some(names_value) = listen_fd_names {
        let names: Vec<_> = names_value.split(':').collect();

        if names.len() == n {
            return Ok((SD_FD_OFFSET..)
                .take(n)
                .zip(names)
                .map(|(fd, name)| SystemDSocket::new(name, fd))
                .collect());
        } else if !names.is_empty() {
            tracing::warn!("Invalid ${}={}", LISTEN_FDNAMES, names_value);
        };
    };

    Ok((SD_FD_OFFSET..)
        .take(n)
        .map(SystemDSocket::unnamed)
        .collect())
}

/// Represents a socket that systemd has passed to us
#[derive(Debug)]
pub struct SystemDSocket {
    name: Option<String>,
    fd: RawFd,
}

impl SystemDSocket {
    fn new<S: Into<String>>(name: S, fd: RawFd) -> Self {
        Self {
            name: Some(name.into()),
            fd,
        }
    }

    fn unnamed(fd: RawFd) -> Self {
        Self { name: None, fd }
    }

    /// Get the name of the socket, if it has one.
    ///
    /// Systemd can provide names in environemnt variables, but it is not required
    /// to. If the socket does not have a name, this will return `None`.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Convert this socket into a `TcpListener`
    pub fn listener(self) -> Result<TcpListener, SocketError> {
        // Safety: This is how systemd rolls
        // See: sd_listen_fds(3), the c API for accessing systemd sockets
        let file = unsafe { File::from_raw_fd(self.fd) };
        let metadata = file.metadata()?;
        if !metadata.file_type().is_socket() {
            return Err(SocketError::NotSocket(file.into_raw_fd()));
        }

        //Todo: We could manually check that this is an INET socket
        // here, so that we don't listen on some arbitrary socket?

        // Safety: Above, we know that the FD is one we should be reading,
        // and we just checked that the socket was one which is listening
        // over tcp;
        let listener = unsafe { TcpListener::from_raw_fd(file.into_raw_fd()) };
        listener.set_nonblocking(true)?;
        Ok(listener)
    }
}

impl AsRawFd for SystemDSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl AsFd for SystemDSocket {
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.fd) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_variables() {
        let listen_fds = "3";
        let listen_fd_names = "alice:bob:charlie";

        let sockets = construct_sockets(
            listen_fds,
            Some(listen_fd_names),
            &format!("{}", process::id()),
        )
        .unwrap();

        let names: Vec<_> = sockets.iter().map(|s| s.name().unwrap()).collect();
        assert_eq!(names, vec!["alice", "bob", "charlie"]);

        let fds: Vec<_> = sockets.iter().map(|s| s.fd).collect();
        assert_eq!(fds, vec![3, 4, 5]);
    }
}
