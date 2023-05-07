/*!
Signaling messages exchanged between used by `NetworkManagers` and signaling server
to facilitate communication in many-to-many topology.
*/

use serde::{Deserialize, Serialize};

use crate::common::IceCandidate;
use crate::{SessionId, UserId};

/// `Enum` consisting of two main categories are messages used to setup signaling session
/// and messages used to setup `WebRTC` connection afterwards.
/// Most of the include [`SessionId`] and [`UserId`] to uniquely identify each peer.
#[derive(Debug, Serialize, Deserialize)]
pub enum SignalMessage {
    /// Either client or server connecting to signaling session
    SessionJoin(SessionId),

    /// Report back to the users that both of them are in session
    SessionReady(SessionId, UserId),

    /// `SDP` Offer that gets passed to the other user without modifications
    SdpOffer(SessionId, UserId, String),

    /// `SDP` Answer that gets passed to the other user without modifications
    SdpAnswer(SessionId, UserId, String),

    /// Proposed ICE Candidate of one user passed to the other user without modifications
    IceCandidate(SessionId, UserId, IceCandidate),

    /// Generic error containing detailed information about the cause
    Error(SessionId, String),
}
