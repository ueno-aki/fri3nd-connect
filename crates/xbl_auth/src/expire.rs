use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

#[macro_export]
macro_rules! now_secs {
    () => {
        ::std::time::SystemTime::now()
            .duration_since(::std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    };
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Expire<V> {
    expired_at: u64,
    data: V,
}

impl<V> Expire<V> {
    #[inline]
    pub fn with_duration(data: V, expired_in: u64) -> Self {
        Self {
            expired_at: expired_in + now_secs!(),
            data,
        }
    }

    #[inline]
    pub fn with_timestamp(data: V, expired_at: u64) -> Self {
        Self { expired_at, data }
    }

    #[inline]
    pub fn is_expired(&self) -> bool {
        self.expired_at <= now_secs!() + 10
    }

    #[inline]
    pub fn take(self) -> V {
        self.data
    }
}

impl<T> Deref for Expire<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl<T> DerefMut for Expire<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T> AsRef<T> for Expire<T> {
    fn as_ref(&self) -> &T {
        &self.data
    }
}
impl<T> AsMut<T> for Expire<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.data
    }
}
