use std::marker::PhantomData;

use serde::{de::Visitor, Deserialize};

use crate::status::Status;

#[derive(Debug)]
pub enum MessageType {
    Subscribe = 1,
    Unsubscribe = 2,
    Event = 3,
    Resync = 4,
}
impl From<u64> for MessageType {
    fn from(value: u64) -> Self {
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
pub struct MessageData<T> {
    pub message_type: MessageType,
    pub sequence_id: u64,
    pub status: Status,
    pub subscription_id: u64,
    pub payload: T,
}
impl<'de, T> Deserialize<'de> for MessageData<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ValueVisitor<D> {
            _marker: PhantomData<D>,
        }
        impl<'de, D> Visitor<'de> for ValueVisitor<D>
        where
            D: Deserialize<'de>,
        {
            type Value = MessageData<D>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("any valid JSON value")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                use serde::de::Error;
                let msg_type = seq.next_element()?.ok_or(Error::invalid_length(0, &self))?;
                let sequence_id = seq.next_element()?.ok_or(Error::invalid_length(1, &self))?;
                let status = seq.next_element()?.ok_or(Error::invalid_length(2, &self))?;
                let subscription_id = seq.next_element()?.ok_or(Error::invalid_length(3, &self))?;
                let payload = seq.next_element()?.ok_or(Error::invalid_length(4, &self))?;
                Ok(MessageData {
                    message_type: <MessageType as From<u64>>::from(msg_type),
                    sequence_id,
                    status: <Status as From<u64>>::from(status),
                    subscription_id,
                    payload,
                })
            }
        }
        deserializer.deserialize_seq(ValueVisitor {
            _marker: PhantomData,
        })
    }
}
