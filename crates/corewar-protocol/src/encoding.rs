use crate::{ClientMessage, ProtocolError, ServerMessage};

/// Encode a server message as MessagePack.
pub fn encode_message(msg: &ServerMessage) -> Vec<u8> {
    rmp_serde::to_vec_named(msg).expect("server messages should always encode to MessagePack")
}

/// Decode a client message from MessagePack.
pub fn decode_client_message(data: &[u8]) -> Result<ClientMessage, ProtocolError> {
    rmp_serde::from_slice(data).map_err(ProtocolError::Decode)
}

/// Encode a server message as JSON for debugging and fallbacks.
pub fn encode_json(msg: &ServerMessage) -> String {
    serde_json::to_string(msg).expect("server messages should always encode to JSON")
}

/// Decode a client message from JSON.
pub fn decode_json(data: &str) -> Result<ClientMessage, ProtocolError> {
    serde_json::from_str(data).map_err(ProtocolError::JsonDecode)
}

#[cfg(test)]
mod tests {
    use super::{decode_client_message, decode_json, encode_json, encode_message};
    use crate::{
        CellInfo, ClientMessage, CycleEvent, InstanceInfo, InstanceStatus, ServerMessage,
        PROTOCOL_VERSION,
    };

    #[test]
    fn messagepack_round_trip_decodes_client_message() {
        let original = ClientMessage::LoadWarrior {
            source: "MOV 0, 1".to_string(),
        };

        let encoded = rmp_serde::to_vec_named(&original).unwrap();
        let decoded = decode_client_message(&encoded).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn json_round_trip_decodes_client_message() {
        let original = ClientMessage::ListInstances;
        let encoded = serde_json::to_string(&original).unwrap();

        let decoded = decode_json(&encoded).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn server_message_encodes_to_messagepack() {
        let msg = ServerMessage::CoreSnapshot {
            instance_id: "arena-1".to_string(),
            cells: vec![CellInfo {
                address: 12,
                owner: Some(7),
                instruction_summary: "MOV 0, 1".to_string(),
            }],
        };

        let encoded = encode_message(&msg);
        let decoded: ServerMessage = rmp_serde::from_slice(&encoded).unwrap();

        assert_eq!(decoded, msg);
    }

    #[test]
    fn server_message_encodes_to_json() {
        let msg = ServerMessage::InstanceList {
            instances: vec![InstanceInfo {
                id: "arena-1".to_string(),
                warrior_names: vec!["Imp".to_string(), "Dwarf".to_string()],
                core_size: 8000,
                cycle: 42,
                status: InstanceStatus::Running,
            }],
        };

        let encoded = encode_json(&msg);

        assert!(encoded.contains("InstanceList"));
        assert!(encoded.contains("arena-1"));
    }

    #[test]
    fn protocol_version_helper_uses_current_version() {
        let mismatch = crate::ProtocolError::version_mismatch(PROTOCOL_VERSION + 1);

        match mismatch {
            crate::ProtocolError::VersionMismatch { expected, actual } => {
                assert_eq!(expected, PROTOCOL_VERSION);
                assert_eq!(actual, PROTOCOL_VERSION + 1);
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn read_cycle_event_is_serializable() {
        let event = CycleEvent::Read {
            address: 99,
            warrior_id: 3,
        };

        let encoded = serde_json::to_string(&event).unwrap();
        let decoded: CycleEvent = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, event);
    }
}
