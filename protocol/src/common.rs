use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Unique identifier of signaling session that each user provides
/// when communicating with the signaling server.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct SessionId(u128);

impl SessionId {
    /// Wrap String into a `SessionId` `struct`
    #[must_use]
    pub const fn new(inner: u128) -> Self {
        Self(inner)
    }

    /// Acquire the underlying type
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn inner(&self) -> u128 {
        self.0
    }
}

impl FromStr for SessionId {
    type Err = <u128 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SessionId({})", self.0)
    }
}

/// Unique identifier of each peer connected to signaling server
/// useful when communicating in one-to-many and many-to-many .
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct UserId(u64);

impl UserId {
    /// Wrap `u64` into a `UserId` `struct`
    #[must_use]
    pub const fn new(inner: u64) -> Self {
        Self(inner)
    }

    /// Acquire the underlying type
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn into_inner(self) -> u64 {
        self.0
    }
}

impl From<u64> for UserId {
    fn from(val: u64) -> Self {
        Self(val)
    }
}

impl Display for UserId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier specifying which peer is host and will be creating an offer,
/// and which will await it.
pub type IsHost = bool;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_m_line_index: Option<u16>,
}
