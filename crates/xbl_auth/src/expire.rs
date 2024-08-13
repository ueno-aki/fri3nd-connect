use anyhow::{anyhow, Result};
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
        self.expired_at <= now_secs!()
    }

    #[inline]
    pub fn get(&self) -> &V {
        self.try_get().unwrap()
    }
    #[inline]
    pub fn get_mut(&mut self) -> &mut V {
        self.try_get_mut().unwrap()
    }

    #[inline]
    pub fn try_get(&self) -> Result<&V> {
        if self.is_expired() {
            Err(anyhow!("This value expired yet."))
        } else {
            Ok(&self.data)
        }
    }
    #[inline]
    pub fn try_get_mut(&mut self) -> Result<&mut V> {
        if self.is_expired() {
            Err(anyhow!("This value expired yet."))
        } else {
            Ok(&mut self.data)
        }
    }

    #[inline]
    pub fn take(self) -> V {
        self.data
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::sleep;

    use super::Expire;

    #[tokio::test]
    async fn test_expired_value() {
        let now = Expire::with_timestamp(String::from("Expire"), 1 + now_secs!());
        assert_eq!(now.try_get().ok(), Some(&String::from("Expire")));
        sleep(Duration::from_secs(1)).await;
        assert!(now.try_get().is_err());
    }
}
