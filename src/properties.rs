//! Access properties of systemd units via systemctl

use std::{collections::HashMap, io, str::FromStr};

use thiserror::Error;

/// Use `systemctl show` to get properties of a systemd unit.
pub fn properties(unit: &str) -> Result<SystemDProperties, PropertyParseError> {
    let mut cmd = std::process::Command::new("systemctl");
    cmd.arg("show");
    cmd.arg(unit);

    let output = cmd.output()?;

    String::from_utf8(output.stdout).unwrap().parse()
}

/// The active state of a systemd unit
#[derive(Debug, Clone, Copy)]
pub enum ActiveState {
    /// The service is active and responding
    Active,

    /// Systemd is reloading the service
    Reloading,

    /// The service is inactive
    Inactive,

    /// The service has failed
    Failed,

    /// The service is activating - it has started but is not yet active
    Activating,

    /// The service is deactivating - it has stopped but is not yet inactive
    Deactivating,
}

/// Errors that can occur when parsing a systemd unit's properties
#[derive(Debug, Error)]
#[error("{0} is not a valid state")]
pub struct StateParseError(String);

impl FromStr for ActiveState {
    type Err = StateParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ActiveState::*;
        match s {
            "active" => Ok(Active),
            "reloading" => Ok(Reloading),
            "inactive" => Ok(Inactive),
            "failed" => Ok(Failed),
            "activating" => Ok(Activating),
            "deactivating" => Ok(Deactivating),
            _ => Err(StateParseError(s.into())),
        }
    }
}

/// Errors that can occur when trying to access systemd unit properties
#[derive(Debug, Error)]
pub enum PropertyParseError {
    /// An invalid state was returned
    #[error(transparent)]
    State(#[from] StateParseError),

    /// A variable is missing the '=' delimiter
    #[error("Line {0} is missing the delimiter '='")]
    MissingDelimeter(String),

    /// An expected property is missing
    #[error("Missing property {0}")]
    MissingProperty(&'static str),

    /// A command error occured running systemctl
    #[error("Running systemctl: {0}")]
    CommandError(#[from] io::Error),
}

/// A map of systemd properties
#[derive(Debug, Clone)]
pub struct SystemDProperties {
    properties: HashMap<String, String>,
    active: ActiveState,
}

impl SystemDProperties {
    /// Get the active state of the systemd unit
    pub fn state(&self) -> ActiveState {
        self.active
    }

    /// Get a property of the systemd unit
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties.get(name).map(|s| s.as_str())
    }
}

impl FromStr for SystemDProperties {
    type Err = PropertyParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut properties = HashMap::new();

        for line in s.lines() {
            let (key, value) = line
                .trim()
                .split_once('=')
                .ok_or_else(|| PropertyParseError::MissingDelimeter(line.into()))?;

            properties.insert(key.to_owned(), value.to_owned());
        }

        let active = properties
            .get("ActiveState")
            .ok_or(PropertyParseError::MissingProperty("ActiveState"))?
            .parse()?;

        Ok(Self { properties, active })
    }
}
