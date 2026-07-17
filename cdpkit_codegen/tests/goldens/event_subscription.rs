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
