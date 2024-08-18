use std::path::PathBuf;

use anyhow::Result;
use cache::Cache;
use crypto::ProofKey;
use expire::Expire;
use msa_live::{MSATokenResponce, MsaAuthFlow};
use p256::ecdsa::SigningKey;
use rand::thread_rng;
use request_token::{
    xbox_device_token::XboxDeviceTokenRequest, xbox_title_token::XboxTitleTokenRequest,
    xbox_user_token::XboxUserTokenRequest, xsts_token::XstsTokenRequest, DeviceToken,
    SignedRequestToken, TitleToken, UserToken, XSTSToken,
};
use reqwest::Client;

pub mod cache;
pub mod crypto;
pub mod expire;
pub mod msa_live;
pub mod request_token;

#[derive(Debug)]
pub struct XBLAuth {
    pub user_name: String,
    cache: Cache,
    client: Client,
    signing_key: SigningKey,
    msa_token: Option<Expire<MSATokenResponce>>,
}

impl XBLAuth {
    pub fn new(cache_path: PathBuf, user_name: String) -> Self {
        let signing_key = SigningKey::random(&mut thread_rng());
        let client = Client::new();
        let cache = Cache::new(cache_path, &user_name);
        Self {
            user_name,
            cache,
            client,
            signing_key,
            msa_token: None,
        }
    }

    pub async fn get_xbox_token(&mut self) -> Result<Expire<XSTSToken>> {
        let ret = match self.cache.get_xsts().await {
            Ok(xsts_cache) if !xsts_cache.is_expired() => return Ok(xsts_cache),
            _ => {
                let proofkey = ProofKey::from(*self.signing_key.verifying_key());
                let user = self.get_user_token().await?;
                let device = self.get_device_token(&proofkey).await?;
                let title = self
                    .get_title_token(device.token.clone(), &proofkey)
                    .await?;
                let xsts = XstsTokenRequest::new(user, device, title, &proofkey)
                    .request_token(&self.signing_key, self.client.clone())
                    .await?;
                XSTSToken::from_response_token(xsts)?
            }
        };
        self.cache.update_xsts(&ret).await?;
        Ok(ret)
    }

    async fn get_user_token(&mut self) -> Result<UserToken> {
        XboxUserTokenRequest::new(self.fetch_access_token().await?)
            .request_token(&self.signing_key, self.client.clone())
            .await
    }
    async fn get_device_token(&self, proofkey: &ProofKey) -> Result<DeviceToken> {
        XboxDeviceTokenRequest::new(proofkey)
            .request_token(&self.signing_key, self.client.clone())
            .await
    }
    async fn get_title_token(
        &mut self,
        device_token: String,
        proofkey: &ProofKey,
    ) -> Result<TitleToken> {
        XboxTitleTokenRequest::new(self.fetch_access_token().await?, device_token, proofkey)
            .request_token(&self.signing_key, self.client.clone())
            .await
    }

    async fn fetch_access_token(&mut self) -> Result<String> {
        let msa_token = match &self.msa_token {
            Some(msa) if !msa.is_expired() => return Ok(msa.access_token.to_owned()),
            Some(msa) => match self.refresh_msa_token(&msa.refresh_token).await {
                Ok(v) => v,
                Err(_) => {
                    println!("Failed to refresh the MSAToken.");
                    self.auth_device_code().await?
                }
            },
            None => self.get_msa_cache().await?,
        };
        let ret = msa_token.access_token.to_owned();
        self.cache.update_msa(&msa_token).await?;
        self.msa_token = Some(msa_token);
        Ok(ret)
    }

    async fn get_msa_cache(&self) -> Result<Expire<MSATokenResponce>> {
        match self.cache.get_msa().await {
            Ok(msa) if !msa.is_expired() => Ok(msa),
            Ok(msa) => match self.refresh_msa_token(&msa.refresh_token).await {
                m @ Ok(..) => m,
                Err(_) => {
                    println!("Failed to refresh the MSAToken.");
                    self.auth_device_code().await
                }
            },
            Err(_) => self.auth_device_code().await,
        }
    }

    async fn auth_device_code(&self) -> Result<Expire<MSATokenResponce>> {
        let responce = self.start_msa_auth().await?;
        println!(
            "Open the page \"{}?otc={}\" in a web browser to sign in as {}",
            responce.verification_uri, responce.user_code, self.user_name
        );
        let msa = self.wait_msa_auth(responce).await?;
        Ok(msa)
    }
}
