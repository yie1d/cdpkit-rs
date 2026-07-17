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
