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
