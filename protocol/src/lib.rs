use serde::{Deserialize, Serialize};

pub type SessionId = String;

#[derive(Debug, Serialize, Deserialize)]
pub enum SignalMessage {
    NewConnection(SessionId),
    SdpOffer(String, SessionId),
    SdpAnswer(String, SessionId),
    IceCandidate(String, SessionId),
    Error(String),
}
