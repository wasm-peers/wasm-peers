use serde::{Deserialize, Serialize};

/// Unique identifier of signaling session that each user provides
/// when communicating with the signaling server
pub type SessionId = String;
pub type IsHost = bool;

/// Enum used by all Client, Mini-server and Signaling server to communicate with each other
/// Two main categories are messages used to setup signaling session
/// and messages used to setup WebRTC connection afterwards
#[derive(Debug, Serialize, Deserialize)]
pub enum SignalMessage {
    /// Either client or server connecting to signaling session
    SessionStartOrJoin(SessionId),
    /// Report back to the users that both of them are in session
    SessionReady(SessionId, IsHost),

    /// SDP Offer that gets passed to the other user without modifications
    SdpOffer(String, SessionId),
    /// SDP Answer that gets passed to the other user without modifications
    SdpAnswer(String, SessionId),
    /// Proposed ICE Candidate of one user passed to the other user without modifications
    IceCandidate(String, SessionId),

    /// Generic error containing detailed information about the cause
    Error(String, SessionId),
}
