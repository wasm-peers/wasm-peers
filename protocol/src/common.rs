use std::convert::Infallible;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Unique identifier of signaling session that each user provides
/// when communicating with the signaling server.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Wrap String into a `SessionId` `struct`
    #[must_use]
    pub const fn new(inner: String) -> Self {
        Self(inner)
    }

    /// Return reference to the underling string
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Acquire the underlying type
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl FromStr for SessionId {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier of each peer connected to signaling server
/// useful when communicating in one-to-many and many-to-many .
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct UserId(usize);

impl UserId {
    /// Wrap `usize` into a `UserId` `struct`
    #[must_use]
    pub const fn new(inner: usize) -> Self {
        Self(inner)
    }

    /// Acquire the underlying type
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn into_inner(self) -> usize {
        self.0
    }
}

impl From<usize> for UserId {
    fn from(val: usize) -> Self {
        Self(val)
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