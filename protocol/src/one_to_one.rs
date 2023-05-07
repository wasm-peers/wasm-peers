/*!
Signaling messages exchanged between used by `MiniServer`, `MiniClient` and signaling server
to facilitate communication in client-server topology.
 */

use serde::{Deserialize, Serialize};

use crate::common::IceCandidate;
use crate::{IsHost, SessionId};

/// `Enum` consisting of two main categories are messages used to setup signaling session
/// and messages used to setup `WebRTC` connection afterwards.
/// All of the messages include [`SessionId`] which is enough to identify the other peer in the connection.
#[derive(Debug, Serialize, Deserialize)]
pub enum SignalMessage {
    /// Either client or server connecting to signaling session
    SessionJoin(SessionId),
    /// Report back to the users that both of them are in session
    SessionReady(SessionId, IsHost),

    /// `SDP` Offer that gets passed to the other user without modifications
    SdpOffer(SessionId, String),
    /// `SDP` Answer that gets passed to the other user without modifications
    SdpAnswer(SessionId, String),
    /// Proposed ICE Candidate of one user passed to the other user without modifications
    IceCandidate(SessionId, IceCandidate),

    /// Generic error containing detailed information about the cause
    Error(SessionId, String),
}
