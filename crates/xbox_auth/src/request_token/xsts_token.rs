use serde::{Deserialize, Serialize};

use crate::{
    crypto::ProofKey,
    request_token::{_inner::headers, generate_signature},
};

use super::{DeviceToken, ResponseToken, SignedRequestToken, TitleToken, UserToken};

#[derive(Debug)]
pub struct XstsTokenRequest<'a> {
    user_token: UserToken,
    device_token: DeviceToken,
    title_token: TitleToken,
    proofkey: &'a ProofKey,
}

impl XstsTokenRequest<'_> {
    pub const XSTS_REQUEST_URL: &'static str = "https://xsts.auth.xboxlive.com/xsts/authorize";
    #[inline]
    pub fn new(
        user_token: UserToken,
        device_token: DeviceToken,
        title_token: TitleToken,
        proofkey: &ProofKey,
    ) -> XstsTokenRequest<'_> {
        XstsTokenRequest {
            user_token,
            device_token,
            title_token,
            proofkey,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct XstsDisplayClaims {
    pub xui: [XstsClaim; 1],
}
#[derive(Debug, Serialize, Deserialize)]
pub struct XstsClaim {
    pub gtg: String,
    pub xid: String,
    pub uhs: String,
}

impl SignedRequestToken for XstsTokenRequest<'_> {
    type DisplayClaims = XstsDisplayClaims;

    async fn request_token(
        &self,
        signer: &p256::ecdsa::SigningKey,
        client: reqwest::Client,
    ) -> anyhow::Result<ResponseToken<Self::DisplayClaims>> {
        pub const RELYING_PARTY: &str = "http://xboxlive.com";
        let body = format!(
            r#"{{
            "Properties": {{
                "UserTokens": ["{}"],
                "DeviceToken": "{}",
                "TitleToken": "{}",
                "ProofKey": {},
                "SandboxId": "RETAIL"
            }},
            "RelyingParty": "{RELYING_PARTY}",
            "TokenType": "JWT"
        }}"#,
            self.user_token.token,
            self.device_token.token,
            self.title_token.token,
            serde_json::to_string(&self.proofkey)?
        );
        let sig = generate_signature(signer, &Self::XSTS_REQUEST_URL.parse()?, &body)?;
        let headers = headers! {
            ("Cache-Control", "no-store, must-revalidate, no-cache"),
            ("x-xbl-contract-version", "1"),
            ("Signature", &sig)
        };
        let ret = client
            .post(Self::XSTS_REQUEST_URL)
            .headers(headers)
            .body(body)
            .send()
            .await?
            .json()
            .await?;
        Ok(ret)
    }
}
