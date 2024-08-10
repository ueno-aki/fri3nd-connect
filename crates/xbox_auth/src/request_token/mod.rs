use std::{fmt::Debug, io::Write};

use _inner::null_terminated;
use anyhow::Result;
use base64::prelude::*;
use byteorder::{WriteBytesExt, BE};
use chrono::DateTime;
use p256::ecdsa::{signature::RandomizedDigestSigner, Signature, SigningKey};
use rand::thread_rng;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use xbox_device_token::XDeviceDisplayClaims;
use xbox_title_token::XTitleDisplayClaims;
use xbox_user_token::XUserDisplayClaims;
use xsts_token::{XstsClaim, XstsDisplayClaims};

use crate::{expire::Expire, now_secs};

pub mod xbox_device_token;
pub mod xbox_title_token;
pub mod xbox_user_token;
pub mod xsts_token;

pub type UserToken = ResponseToken<XUserDisplayClaims>;
pub type DeviceToken = ResponseToken<XDeviceDisplayClaims>;
pub type TitleToken = ResponseToken<XTitleDisplayClaims>;

#[derive(Debug, Serialize, Deserialize)]
pub struct XSTSToken {
    pub gamer_tag: String,
    pub xuid: String,
    pub user_hash: String,
    pub token: String,
}
impl XSTSToken {
    pub fn from_response_token(value: ResponseToken<XstsDisplayClaims>) -> Result<Expire<Self>> {
        let expired_at = DateTime::parse_from_rfc3339(&value.not_after)?.timestamp();
        let [XstsClaim { gtg, uhs, xid }] = value.display_claims.xui;
        let xsts_token = Self {
            gamer_tag: gtg,
            user_hash: uhs,
            xuid: xid,
            token: value.token,
        };
        Ok(Expire::with_timestamp(xsts_token, expired_at as u64))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ResponseToken<T: Debug> {
    pub issue_instant: String,
    pub not_after: String,
    pub token: String,
    pub display_claims: T,
}

pub trait SignedRequestToken {
    type DisplayClaims: Debug;
    fn request_token(
        &self,
        signer: &SigningKey,
        client: Client,
    ) -> impl std::future::Future<Output = Result<ResponseToken<Self::DisplayClaims>>> + Send;
}

pub fn generate_signature(signer: &SigningKey, url: &Url, payload: &str) -> Result<String> {
    const SEC_TO_NT_TIME_EPOCH: u64 = 11_644_473_600; // UNIX_TIME_EPOCH - NT_TIME_EPOCH
    let filetime = (now_secs!() + SEC_TO_NT_TIME_EPOCH) * 10_000_000;
    let uri = url.path();

    let mut buf: Vec<u8> = vec![];
    null_terminated! ((buf) => {
        buf.write_i32::<BE>(1)? // Policy Version
        buf.write_u64::<BE>(filetime)?
        buf.write_all(b"POST")?
        buf.write_all(uri.as_bytes())?
        buf.write_all(b"")? // AuthorizationToken
        buf.write_all(payload.as_bytes())?
    });

    let mut digest = sha2::Sha256::new();
    digest.update(buf);
    let signature: Signature = signer.sign_digest_with_rng(&mut thread_rng(), digest);

    let mut ret: Vec<u8> = vec![];
    ret.write_i32::<BE>(1)?; // Policy Version
    ret.write_u64::<BE>(filetime)?;
    ret.write_all(&signature.to_vec())?;
    Ok(BASE64_STANDARD.encode(ret))
}

pub(crate) mod _inner {
    pub(crate) trait HeaderMapImpl {
        fn insert_pair(&mut self, pair: (&'static str, &str)) -> anyhow::Result<()>;
    }
    impl HeaderMapImpl for reqwest::header::HeaderMap {
        #[inline]
        fn insert_pair(&mut self, (key, val): (&'static str, &str)) -> anyhow::Result<()> {
            self.insert(key, reqwest::header::HeaderValue::from_str(val)?);
            Ok(())
        }
    }
    macro_rules! headers {
        ($($v:expr),*) => {{
            use crate::request_token::_inner::HeaderMapImpl;
            let mut _headers = ::reqwest::header::HeaderMap::new();
            $(
                _headers.insert_pair($v).expect("InvalidHeaderValue");
            )*
            _headers
        }};
    }
    pub(crate) use headers;

    macro_rules! null_terminated {
        ( ($terminated:expr) => { $($v:stmt)+ } ) => {
            $(
                $v
                $terminated.write_all(&[0])?;
            )*
        };
    }
    pub(super) use null_terminated;
}
