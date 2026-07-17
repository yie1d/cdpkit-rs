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
