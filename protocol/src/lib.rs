use serde::{Deserialize, Serialize};

type SessionId = String;
type StdOffer = String;

#[derive(Debug, Serialize, Deserialize)]
pub enum SignalMessage {
    SdpOffer(String, SessionId),
    SdpAnswer(String, SessionId),
    IceCandidate(String, SessionId),
    IceError(String, SessionId),
}
