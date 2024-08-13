use p256::{ecdsa::VerifyingKey, elliptic_curve::JwkEcKey, PublicKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofKey {
    pub alg: String,
    pub r#use: String,
    #[serde(flatten)]
    pub jwk: JwkEcKey,
}

// p-256
impl From<VerifyingKey> for ProofKey {
    fn from(value: VerifyingKey) -> Self {
        let jwk = PublicKey::from(value).to_jwk();
        Self {
            alg: "ES256".into(),
            r#use: "sig".into(),
            jwk,
        }
    }
}
