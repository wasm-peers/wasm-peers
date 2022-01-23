/*!
Helper crate that declares common types and structures shared between [rusty-games-library](../rusty_games_library/index.html)
and [rusty-games-signaling-server](../rusty_games_signaling_server/index.html).
*/

#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::str::FromStr;

pub mod many_to_many;
pub mod one_to_many;
pub mod one_to_one;

/// Unique identifier of signaling session that each user provides
/// when communicating with the signaling server.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Wrap String into a SessionId struct
    pub fn new(inner: String) -> Self {
        SessionId(inner)
    }

    /// Return reference to the underling string
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// Acquire the underlying type
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl FromStr for SessionId {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SessionId(s.to_string()))
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier of each peer connected to signaling server
/// useful when communicating in one-to-many and many-to-many topologies.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct UserId(usize);

impl UserId {
    /// Wrap usize into a UserId struct
    pub fn new(inner: usize) -> Self {
        UserId(inner)
    }

    /// Acquire the underlying type
    pub fn into_inner(self) -> usize {
        self.0
    }
}

impl From<usize> for UserId {
    fn from(val: usize) -> Self {
        UserId(val)
    }
}

impl Display for UserId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for UserId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Unique identifier specifying which peer is host and will be creating an offer,
/// and which will await it.
pub type IsHost = bool;
