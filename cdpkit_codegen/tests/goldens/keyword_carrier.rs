        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct KeywordCarrier {
            #[serde(rename = "type")]
            pub type_: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "override")]
            pub override_: Option<bool>,
        }
