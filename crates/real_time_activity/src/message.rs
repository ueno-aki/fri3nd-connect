use serde::{de::Visitor, Deserialize};

use crate::status::Status;

#[derive(Debug)]
pub enum MessageType {
    Subscribe = 1,
    Unsubscribe = 2,
    Event = 3,
    Resync = 4,
}
impl From<i64> for MessageType {
    fn from(value: i64) -> Self {
        match value {
            1 => Self::Subscribe,
            2 => Self::Unsubscribe,
            3 => Self::Event,
            4 => Self::Resync,
            n => panic!("[<MessageType as From<u64>>::from] Invalid value: {n}."),
        }
    }
}

#[derive(Debug)]
pub enum MessageData {
    Subscribe {
        seq_id: i64,
        status: Status,
        sub_id: i64,
        connection_id: String,
    },
    Unsubscribe {
        seq_id: i64,
        status: Status,
    },
    Event {
        sub_id: i64,
        data: EventData,
    },
    Resync,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EventData {
    pub ncid: String,
    pub shoulder_taps: Vec<EventShoulderTap>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EventShoulderTap {
    pub timestamp: String,
    pub subscription: String,
    pub resource_type: String,
    pub resource: String,
    pub branch: String,
    pub change_number: i64,
}

impl<'de> Deserialize<'de> for MessageData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct MessageVisitor;
        impl<'de> Visitor<'de> for MessageVisitor {
            type Value = MessageData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("any valid JSON value")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                use serde::de::Error;
                let msg_type = <MessageType as From<i64>>::from(
                    seq.next_element()?.ok_or(Error::invalid_length(0, &self))?,
                );
                let ret = match msg_type {
                    MessageType::Subscribe => {
                        let seq_id = seq.next_element()?.ok_or(Error::invalid_length(1, &self))?;
                        let status = seq.next_element()?.ok_or(Error::invalid_length(2, &self))?;
                        let sub_id = seq.next_element()?.ok_or(Error::invalid_length(3, &self))?;
                        #[derive(Debug, Deserialize)]
                        #[allow(non_snake_case)]
                        struct Payload {
                            ConnectionId: String,
                        }
                        let Payload { ConnectionId } =
                            seq.next_element()?.ok_or(Error::invalid_length(4, &self))?;
                        MessageData::Subscribe {
                            seq_id,
                            status: <Status as From<i64>>::from(status),
                            sub_id,
                            connection_id: ConnectionId,
                        }
                    }
                    MessageType::Unsubscribe => {
                        let seq_id = seq.next_element()?.ok_or(Error::invalid_length(1, &self))?;
                        let status = seq.next_element()?.ok_or(Error::invalid_length(2, &self))?;
                        MessageData::Unsubscribe {
                            seq_id,
                            status: <Status as From<i64>>::from(status),
                        }
                    }
                    MessageType::Event => {
                        let sub_id = seq.next_element()?.ok_or(Error::invalid_length(1, &self))?;
                        let data = seq.next_element()?.ok_or(Error::invalid_length(2, &self))?;
                        MessageData::Event { sub_id, data }
                    }
                    MessageType::Resync => MessageData::Resync,
                };
                Ok(ret)
            }
        }
        deserializer.deserialize_seq(MessageVisitor)
    }
}
