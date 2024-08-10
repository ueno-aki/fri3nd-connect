use serde::{Deserialize, Serialize};

use crate::{
    crypto::ProofKey,
    request_token::{_inner::headers, generate_signature},
};

use super::SignedRequestToken;

#[derive(Debug)]
pub struct XboxTitleTokenRequest {
    msa_access_token: String,
    device_token: String,
    proofkey: ProofKey,
}

impl XboxTitleTokenRequest {
    pub const TITLE_REQUEST_URL: &'static str =
        "https://title.auth.xboxlive.com/title/authenticate";
    #[inline]
    pub fn new(msa_access_token: String, device_token: String, proofkey: ProofKey) -> Self {
        Self {
            msa_access_token,
            device_token,
            proofkey,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct XTitleDisplayClaims {
    pub xti: XttClaim,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct XttClaim {
    pub tid: String,
}

impl SignedRequestToken for XboxTitleTokenRequest {
    type DisplayClaims = XTitleDisplayClaims;

    async fn request_token(
        &self,
        signer: &p256::ecdsa::SigningKey,
        client: reqwest::Client,
    ) -> anyhow::Result<super::ResponseToken<Self::DisplayClaims>> {
        let body = format!(
            r#"{{
            "Properties": {{
                "AuthMethod": "RPS",
                "DeviceToken": "{}",
                "RpsTicket": "t={}",
                "SiteName": "user.auth.xboxlive.com",
                "ProofKey": {}
            }},
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }}"#,
            self.device_token,
            self.msa_access_token,
            serde_json::to_string(&self.proofkey)?
        );
        let sig = generate_signature(signer, &Self::TITLE_REQUEST_URL.parse()?, &body)?;
        let headers = headers! {
            ("Cache-Control", "no-store, must-revalidate, no-cache"),
            ("x-xbl-contract-version", "1"),
            ("Signature", &sig)
        };
        let ret = client
            .post(Self::TITLE_REQUEST_URL)
            .headers(headers)
            .body(body)
            .send()
            .await?
            .json()
            .await?;
        Ok(ret)
    }
}
