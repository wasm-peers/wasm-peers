/*!

*/

// #![deny(missing_docs)]

use serde::{Deserialize, Serialize};

pub mod one_to_many;
pub mod one_to_one;

/// Unique identifier of signaling session that each user provides
/// when communicating with the signaling server
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct SessionId {
    pub inner: String,
}

impl SessionId {
    pub fn new(inner: String) -> Self {
        SessionId { inner }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct UserId {
    pub inner: usize,
}

impl UserId {
    pub fn new(inner: usize) -> Self {
        UserId { inner }
    }
}

/// Unique identifier specifying which peer is host and will be creating an offer,
/// and which will await it.
pub type IsHost = bool;
