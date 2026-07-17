// Auto-generated from Chrome DevTools Protocol
// DO NOT EDIT MANUALLY  OvO

#![allow(dead_code, unused_imports, clippy::all)]

use serde::{Deserialize, Serialize};
use crate::Method;

/// CDP Protocol version (major.minor)
pub const CDP_VERSION: &str = "1.3";

pub mod runtime {
    use super::*;

    /// CDP type definitions for this domain.
    pub mod types {
        use super::super::*;

        pub type RemoteObjectId = String;

    }

}

pub mod target {
    use super::*;

    /// CDP type definitions for this domain.
    pub mod types {
        use super::super::*;

        pub type TargetID = String;

        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum Mode {
            #[serde(rename = "default")]
            Default,
            #[serde(rename = "override")]
            Override_,
        }
        impl AsRef<str> for Mode {
            fn as_ref(&self) -> &str {
                match self {
                    Self::Default => "default",
                    Self::Override_ => "override",
                }
            }
        }
        impl std::str::FromStr for Mode {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    "default" => Ok(Self::Default),
                    "override" => Ok(Self::Override_),
                    _ => Err(s.to_string()),
                }
            }
        }
        impl std::fmt::Display for Mode {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_ref())
            }
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct KeywordCarrier {
            #[serde(rename = "type")]
            pub type_: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "override")]
            pub override_: Option<bool>,
        }

    }

    /// Response types returned by methods in this domain.
    pub mod responses {
        use super::super::*;
        use super::types;

        #[derive(Debug, Clone, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct AttachToTargetResponse {
            pub session_id: String,
        }

        #[derive(Debug, Clone, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct InspectResponse {
            pub details: types::KeywordCarrier,
        }

    }

    /// CDP methods (commands) for this domain.
    pub mod methods {
        use super::super::*;
        use super::types;
        use super::responses;

        #[derive(Debug, Clone, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct AttachToTarget {
            pub target_id: types::TargetID,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub flatten: Option<bool>,
        }

        impl AttachToTarget {
            pub fn new(target_id: types::TargetID) -> Self {
                Self {
                    target_id,
                    flatten: Some(true),
                }
            }

            pub fn with_flatten(mut self, flatten: bool) -> Self {
                self.flatten = Some(flatten);
                self
            }
            pub async fn send(self, target: &(impl crate::Sender + Sync)) -> Result<responses::AttachToTargetResponse, crate::CdpError> {
                target.send_cmd(self).await
            }
        }

        impl Method for AttachToTarget {
            type Response = responses::AttachToTargetResponse;
            const METHOD: &'static str = "Target.attachToTarget";

            fn validate(&self) -> Result<(), crate::CdpError> {
                if matches!(self.flatten, Some(false)) {
                    return Err(crate::CdpError::UnsupportedConfiguration(
                        "Target.attachToTarget requires flatten=true; cdpkit only supports flattened sessions".to_string(),
                    ));
                }
                Ok(())
            }
        }

        #[derive(Debug, Clone, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct SetAutoAttach {
            pub auto_attach: bool,
            pub wait_for_debugger_on_start: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub flatten: Option<bool>,
        }

        impl SetAutoAttach {
            pub fn new(auto_attach: bool, wait_for_debugger_on_start: bool) -> Self {
                Self {
                    auto_attach,
                    wait_for_debugger_on_start,
                    flatten: Some(true),
                }
            }

            pub fn with_flatten(mut self, flatten: bool) -> Self {
                self.flatten = Some(flatten);
                self
            }
            pub async fn send(self, target: &(impl crate::Sender + Sync)) -> Result<(), crate::CdpError> {
                target.send_cmd(self).await
            }
        }

        impl Method for SetAutoAttach {
            type Response = ();
            const METHOD: &'static str = "Target.setAutoAttach";

            fn validate(&self) -> Result<(), crate::CdpError> {
                if matches!(self.flatten, Some(false)) {
                    return Err(crate::CdpError::UnsupportedConfiguration(
                        "Target.setAutoAttach requires flatten=true; cdpkit only supports flattened sessions".to_string(),
                    ));
                }
                Ok(())
            }
        }

        #[derive(Debug, Clone, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct Inspect {
            pub remote_object_id: runtime::types::RemoteObjectId,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "type")]
            pub type_: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub mode: Option<types::Mode>,
        }

        impl Inspect {
            pub fn new(remote_object_id: runtime::types::RemoteObjectId) -> Self {
                Self {
                    remote_object_id,
                    type_: None,
                    mode: None,
                }
            }

            pub fn with_type_(mut self, type_: impl Into<String>) -> Self {
                self.type_ = Some(type_.into());
                self
            }

            pub fn with_mode(mut self, mode: types::Mode) -> Self {
                self.mode = Some(mode);
                self
            }
            pub async fn send(self, target: &(impl crate::Sender + Sync)) -> Result<responses::InspectResponse, crate::CdpError> {
                target.send_cmd(self).await
            }
        }

        impl Method for Inspect {
            type Response = responses::InspectResponse;
            const METHOD: &'static str = "Target.inspect";
        }

    }

    /// CDP events for this domain.
    pub mod events {
        use super::super::*;
        use super::types;

        #[derive(Debug, Clone, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct AttachedToTarget {
            pub session_id: String,
            pub target_id: types::TargetID,
            #[serde(rename = "type")]
            pub type_: Option<String>,
        }

        impl AttachedToTarget {
            pub fn subscribe(target: &(impl crate::Sender + Sync)) -> crate::EventStream<Self> {
                target.event_stream("Target.attachedToTarget")
            }

            pub fn subscribe_with_policy(
                target: &(impl crate::Sender + Sync),
                policy: crate::EventStreamPolicy,
            ) -> crate::EventStream<Self> {
                target.event_stream_with_policy("Target.attachedToTarget", policy)
            }

            pub fn subscribe_result(target: &(impl crate::Sender + Sync)) -> crate::EventStreamResult<Self> {
                target.event_stream_result("Target.attachedToTarget")
            }

            pub fn subscribe_result_with_policy(
                target: &(impl crate::Sender + Sync),
                policy: crate::EventStreamPolicy,
            ) -> crate::EventStreamResult<Self> {
                target.event_stream_result_with_policy("Target.attachedToTarget", policy)
            }
        }

    }

}
