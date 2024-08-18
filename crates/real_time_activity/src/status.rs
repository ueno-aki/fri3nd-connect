#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Success = 0,
    UnknownResource = 1,
    SubscriptionLimitReached = 2,
    NoResourceData = 3,
    Throttled = 1001,
    ServiceUnavailable = 1002,
}

impl From<i64> for Status {
    fn from(value: i64) -> Self {
        match value {
            0 => Self::Success,
            1 => Self::UnknownResource,
            2 => Self::SubscriptionLimitReached,
            3 => Self::NoResourceData,
            1001 => Self::Throttled,
            1002 => Self::ServiceUnavailable,
            n => panic!("[<StatusCode as From<u64>>::from] Invalid value: {n}."),
        }
    }
}
