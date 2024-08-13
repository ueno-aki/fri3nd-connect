use p256::ecdsa::SigningKey;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{crypto::ProofKey, request_token::_inner::headers};

use super::{generate_signature, SignedRequestToken};

#[derive(Debug)]
pub struct XboxDeviceTokenRequest<'a> {
    proofkey: &'a ProofKey,
}

impl XboxDeviceTokenRequest<'_> {
    pub const DEVICE_REQUEST_URL: &'static str =
        "https://device.auth.xboxlive.com/device/authenticate";
    #[inline]
    pub fn new(proofkey: &ProofKey) -> XboxDeviceTokenRequest<'_> {
        XboxDeviceTokenRequest { proofkey }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct XDeviceDisplayClaims {
    pub xdi: XdtClaim,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct XdtClaim {
    pub did: String,
    pub dcs: String,
}

impl SignedRequestToken for XboxDeviceTokenRequest<'_> {
    type DisplayClaims = XDeviceDisplayClaims;

    async fn request_token(
        &self,
        signer: &SigningKey,
        client: reqwest::Client,
    ) -> anyhow::Result<super::ResponseToken<Self::DisplayClaims>> {
        let body = format!(
            r#"{{
            "Properties": {{
                "AuthMethod": "ProofOfPossession",
                "Id": "{{{0}}}",
                "SerialNumber": "{{{0}}}",
                "Version": "0.0.0",
                "DeviceType": "Nintendo",
                "ProofKey": {1}
            }},
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }}"#,
            Uuid::new_v4(),
            serde_json::to_string(&self.proofkey)?
        );
        let sig = generate_signature(signer, &Self::DEVICE_REQUEST_URL.parse()?, &body)?;
        let headers = headers! {
            ("Cache-Control", "no-store, must-revalidate, no-cache"),
            ("x-xbl-contract-version", "1"),
            ("Signature", &sig)
        };
        let ret = client
            .post(Self::DEVICE_REQUEST_URL)
            .headers(headers)
            .body(body)
            .send()
            .await?
            .json()
            .await?;
        Ok(ret)
    }
}
