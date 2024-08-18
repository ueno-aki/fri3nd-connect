use crate::message::EventData;

#[derive(Debug)]
pub enum RtaEvent {
    Subscribe {
        seq_id: i64,
        sub_id: i64,
        connection_id: String,
    },
    Unsubscribe {
        seq_id: i64,
    },
    Event {
        sub_id: i64,
        data: EventData,
    },
    Pong(Vec<u8>),
}
