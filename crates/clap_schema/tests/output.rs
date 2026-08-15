//! Runtime JSON emission tests for handler-derived successful outputs.
#![expect(dead_code, reason = "test handlers are reflected rather than executed")]

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use clap_schema::{WriteJsonError, write_json};
    use schemars::JsonSchema;
    use serde::{Serialize, Serializer, ser::Error as _};

    #[derive(Debug, Serialize, JsonSchema)]
    struct Payload {
        value: String,
    }

    #[derive(Debug, JsonSchema)]
    struct SerializationFailure;

    impl Serialize for SerializationFailure {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            Err(S::Error::custom("deliberate serialization failure"))
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    enum HandlerError {
        Failed,
    }

    #[test]
    fn write_json_tracks_success_unit_and_error_paths() {
        let mut output = Vec::new();
        write_json(&mut output, Ok::<_, Infallible>(Payload { value: "ok".to_owned() }))
            .expect("serialize payload");
        assert_eq!(output, br#"{"value":"ok"}"#);

        let mut unit_output = Vec::new();
        write_json(&mut unit_output, Ok::<(), Infallible>(())).expect("serialize unit");
        assert!(unit_output.is_empty());

        let handler_error = write_json(Vec::new(), Err::<Payload, _>(HandlerError::Failed))
            .expect_err("handler failure");
        assert!(matches!(handler_error, WriteJsonError::Handler(HandlerError::Failed)));

        let serialization_error = write_json(Vec::new(), Ok::<_, Infallible>(SerializationFailure))
            .expect_err("serialization failure");
        assert!(matches!(serialization_error, WriteJsonError::Serialize(_)));
    }
}
