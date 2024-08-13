use std::time::Duration;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Instant};

use crate::{expire::Expire, XBLAuth};

const SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";
const SWITCH_CLIENT_ID: &str = "00000000441cc96b";

const LIVE_DEVICE_CODE_REQUEST: &str = "https://login.live.com/oauth20_connect.srf";
const LIVE_ACCESS_TOKEN_REQUEST: &str = "https://login.live.com/oauth20_token.srf";

pub trait MsaAuthFlow {
    fn start_msa_auth(
        &self,
    ) -> impl std::future::Future<Output = Result<DeviceAuthResponse>> + Send;
    fn wait_msa_auth(
        &self,
        auth_response: DeviceAuthResponse,
    ) -> impl std::future::Future<Output = Result<Expire<MSATokenResponce>>> + Send;
    fn refresh_msa_token(
        &self,
        refresh_token: &str,
    ) -> impl std::future::Future<Output = Result<Expire<MSATokenResponce>>> + Send;
}

impl MsaAuthFlow for XBLAuth {
    async fn start_msa_auth(&self) -> Result<DeviceAuthResponse> {
        let ret = self
            .client
            .post(LIVE_DEVICE_CODE_REQUEST)
            .form(&[
                ("scope", SCOPE),
                ("client_id", SWITCH_CLIENT_ID),
                ("response_type", "device_code"),
            ])
            .send()
            .await?
            .json()
            .await?;
        Ok(ret)
    }

    async fn wait_msa_auth(
        &self,
        auth_response: DeviceAuthResponse,
    ) -> Result<Expire<MSATokenResponce>> {
        let expired_at = Instant::now() + Duration::from_secs(auth_response.expires_in);
        let ret = loop {
            if Instant::now() > expired_at {
                break Err(anyhow!("This device code has expired."));
            }
            let response = self
                .client
                .post(LIVE_ACCESS_TOKEN_REQUEST)
                .form(&[
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("device_code", &auth_response.device_code),
                    ("client_id", SWITCH_CLIENT_ID),
                ])
                .send()
                .await?;
            if let Ok(token) = response.json::<MSATokenResponce>().await {
                let expires_in = token.expires_in;
                break Ok(Expire::with_duration(token, expires_in));
            }
            sleep(Duration::from_secs(auth_response.interval)).await;
        }?;
        Ok(ret)
    }

    async fn refresh_msa_token(&self, refresh_token: &str) -> Result<Expire<MSATokenResponce>> {
        let response: MSATokenResponce = self
            .client
            .post(LIVE_ACCESS_TOKEN_REQUEST)
            .form(&[
                ("scope", SCOPE),
                ("grant_type", "refresh_token"),
                ("client_id", SWITCH_CLIENT_ID),
                ("refresh_token", refresh_token),
            ])
            .send()
            .await?
            .json()
            .await?;
        let expired_in = response.expires_in;
        Ok(Expire::with_duration(response, expired_in))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceAuthResponse {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MSATokenResponce {
    pub token_type: String,
    pub scope: String,
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: String,
    pub expires_in: u64,
}
