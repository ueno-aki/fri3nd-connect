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
        }
    }
    pub async fn get_xbox_token(&self) -> Result<Expire<XSTSToken>> {
        let ret = match self.cache.get_xsts().await {
            Ok(xsts_cache) if !xsts_cache.is_expired() => xsts_cache,
            _ => {
                let proofkey = ProofKey::from(*self.signing_key.verifying_key());
                let msa = self.access_msa_token().await?;
                let xut = self.get_user_token(msa.try_get()?).await?;
                let xdt = self.get_device_token(&proofkey).await?;
                let xtt = self
                    .get_title_token(msa.try_get()?, &xdt, &proofkey)
                    .await?;
                let xsts = XstsTokenRequest::new(xut, xdt, xtt, &proofkey)
                    .request_token(&self.signing_key, Client::clone(&self.client))
                    .await?;
                XSTSToken::from_response_token(xsts)?
            }
        };
        self.cache.update_xsts(&ret).await?;
        Ok(ret)
    }

    #[inline]
    async fn get_user_token(&self, msa_token: &MSATokenResponce) -> Result<UserToken> {
        let access_token = msa_token.access_token.clone();
        XboxUserTokenRequest::new(access_token)
            .request_token(&self.signing_key, self.client.clone())
            .await
    }
    #[inline]
    async fn get_device_token(&self, proofkey: &ProofKey) -> Result<DeviceToken> {
        XboxDeviceTokenRequest::new(proofkey)
            .request_token(&self.signing_key, self.client.clone())
            .await
    }
    #[inline]
    async fn get_title_token(
        &self,
        msa_token: &MSATokenResponce,
        device_token: &DeviceToken,
        proofkey: &ProofKey,
    ) -> Result<TitleToken> {
        let access_token = msa_token.access_token.clone();
        XboxTitleTokenRequest::new(access_token, device_token.token.clone(), proofkey)
            .request_token(&self.signing_key, self.client.clone())
            .await
    }

    pub async fn access_msa_token(&self) -> Result<Expire<MSATokenResponce>> {
        let ret = match self.cache.get_msa().await {
            Ok(msa_cache) if !msa_cache.is_expired() => msa_cache,
            Ok(msa_cache) => {
                let refresh_token = msa_cache.take().refresh_token;
                match self.refresh_msa_token(&refresh_token).await {
                    Ok(msa) => msa,
                    Err(_) => self.auth_device_code().await?,
                }
            }
            Err(..) => self.auth_device_code().await?,
        };
        self.cache.update_msa(&ret).await?;
        Ok(ret)
    }
    #[inline]
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anyhow::Result;

    use crate::XBLAuth;

    #[ignore]
    #[tokio::test]
    async fn test_xbl_auth() -> Result<()> {
        let xbl_auth = XBLAuth::new(Path::new("../../auth").to_path_buf(), "Ferris".into());
        let xsts = xbl_auth.get_xbox_token().await?;
        dbg!(&xsts);
        Ok(())
    }
}
