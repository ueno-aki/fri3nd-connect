use anyhow::Result;
use cache::Cache;
use crypto::ProofKey;
use expire::Expire;
use msa_live::{MSATokenResponce, MsaAuthFlow};
use p256::ecdsa::SigningKey;
use rand::thread_rng;
use request_token::{
    xbox_device_token::XboxDeviceTokenRequest, xbox_title_token::XboxTitleTokenRequest,
    xbox_user_token::XboxUserTokenRequest, xsts_token::XstsTokenRequest, SignedRequestToken,
    XSTSToken,
};
use reqwest::Client;
use std::path::Path;

pub mod cache;
pub mod crypto;
pub mod expire;
pub mod msa_live;
pub mod request_token;

pub struct XBLAuth<'a> {
    pub user_name: &'a str,
    cache: Cache<'a>,
    client: Client,
    signing_key: SigningKey,
}

impl<'a> XBLAuth<'a> {
    pub fn new(cache_path: &'a Path, user_name: &'a str) -> Self {
        let signing_key = SigningKey::random(&mut thread_rng());
        let client = Client::new();
        let cache = Cache::new(cache_path, user_name);
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
                let xut = XboxUserTokenRequest::new(msa.try_get()?.access_token.to_owned())
                    .request_token(&self.signing_key, Client::clone(&self.client))
                    .await?;
                let xdt = XboxDeviceTokenRequest::new(&proofkey)
                    .request_token(&self.signing_key, Client::clone(&self.client))
                    .await?;
                let xtt = XboxTitleTokenRequest::new(
                    msa.try_get()?.access_token.to_owned(),
                    xdt.token.clone(),
                    &proofkey,
                )
                .request_token(&self.signing_key, Client::clone(&self.client))
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

    #[tokio::test]
    async fn test_xbl_auth() -> Result<()> {
        let xbl_auth = XBLAuth::new(Path::new("../../auth"), "Ferris");
        let xsts = xbl_auth.get_xbox_token().await?;
        dbg!(&xsts);

        Ok(())
    }
}
