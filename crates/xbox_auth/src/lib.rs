use anyhow::Result;
use cache::Cache;
use expire::Expire;
use msa_live::{MSATokenResponce, MsaAuthFlow};
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
}

impl<'a> XBLAuth<'a> {
    pub fn new(cache_path: &'a Path, user_name: &'a str) -> Self {
        let client = Client::new();
        let cache = Cache::new(cache_path, user_name);
        Self {
            user_name,
            cache,
            client,
        }
    }
    pub async fn access_msa_token(&self) -> Result<Expire<MSATokenResponce>> {
        let ret = match self.cache.get_msa().await {
            Ok(msa_cache) => {
                if !msa_cache.is_expired() {
                    msa_cache
                } else {
                    let refresh_token = msa_cache.take().refresh_token;
                    let msa = self.refresh_msa_token(&refresh_token).await?;
                    self.cache.update_msa(&msa).await?;
                    msa
                }
            }
            Err(..) => {
                let responce = self.start_msa_auth().await?;
                println!(
                    "Open the page \"{}?otc={}\" in a web browser to sign in as {}",
                    responce.verification_uri, responce.user_code, self.user_name
                );
                let msa = self.wait_msa_auth(responce).await?;
                self.cache.update_msa(&msa).await?;
                msa
            }
        };
        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anyhow::Result;
    use p256::ecdsa::SigningKey;
    use rand::thread_rng;
    use reqwest::Client;

    use crate::{
        crypto::ProofKey,
        request_token::{
            xbox_device_token::XboxDeviceTokenRequest, xbox_title_token::XboxTitleTokenRequest,
            xbox_user_token::XboxUserTokenRequest, xsts_token::XstsTokenRequest,
            SignedRequestToken,
        },
        XBLAuth,
    };

    #[tokio::test]
    async fn test_xbl_auth() -> Result<()> {
        let secret = SigningKey::random(&mut thread_rng());
        let proofkey = ProofKey::from(*secret.verifying_key());

        let xbl_auth = XBLAuth::new(Path::new("../../auth"), "Ferris");
        let msa = xbl_auth.access_msa_token().await?;

        let xut_req = XboxUserTokenRequest::new(msa.get().access_token.to_owned());
        let xut = xut_req
            .request_token(&secret, Client::clone(&xbl_auth.client))
            .await?;
        dbg!(&xut);

        let xdt_req = XboxDeviceTokenRequest::new(proofkey.clone());
        let xdt = xdt_req
            .request_token(&secret, Client::clone(&xbl_auth.client))
            .await?;
        dbg!(&xdt);

        let xtt_req = XboxTitleTokenRequest::new(
            msa.get().access_token.to_owned(),
            xdt.token.clone(),
            proofkey.clone(),
        );
        let xtt = xtt_req
            .request_token(&secret, Client::clone(&xbl_auth.client))
            .await?;
        dbg!(&xtt);

        let xsts_req = XstsTokenRequest::new(xut, xdt, xtt, proofkey.clone());
        let xsts = xsts_req
            .request_token(&secret, Client::clone(&xbl_auth.client))
            .await?;
        dbg!(&xsts);
        Ok(())
    }
}
