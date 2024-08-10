use p256::ecdsa::SigningKey;
use serde::{Deserialize, Serialize};

use super::{SignedRequestToken, _inner::headers, generate_signature};

#[derive(Debug)]
pub struct XboxUserTokenRequest {
    msa_access_token: String,
}

impl XboxUserTokenRequest {
    pub const USER_REQUEST_URL: &'static str = "https://user.auth.xboxlive.com/user/authenticate";
    #[inline]
    pub fn new(msa_access_token: String) -> Self {
        Self { msa_access_token }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct XUserDisplayClaims {
    pub xui: [XutClaim; 1],
}
#[derive(Debug, Serialize, Deserialize)]
pub struct XutClaim {
    pub uhs: String,
}

impl SignedRequestToken for XboxUserTokenRequest {
    type DisplayClaims = XUserDisplayClaims;

    async fn request_token(
        &self,
        signer: &SigningKey,
        client: reqwest::Client,
    ) -> anyhow::Result<super::ResponseToken<Self::DisplayClaims>> {
        let body = format!(
            r#"{{
            "Properties": {{
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": "t={}"
            }},
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }}"#,
            self.msa_access_token
        );
        let sig = generate_signature(signer, &Self::USER_REQUEST_URL.parse()?, &body)?;
        let headers = headers! {
            ("Accept", "application/json"),
            ("Content-Type", "application/json"),
            ("x-xbl-contract-version", "2"),
            ("Cache-Control", "no-store, must-revalidate, no-cache"),
            ("Signature", &sig)
        };
        let ret = client
            .post(Self::USER_REQUEST_URL)
            .headers(headers)
            .body(body)
            .send()
            .await?
            .json()
            .await?;
        Ok(ret)
    }
}
