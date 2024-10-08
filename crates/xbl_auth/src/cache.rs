use std::{fs, path::PathBuf};

use anyhow::Result;
use sha2::{Digest, Sha256};
use tokio::{
    fs::{write, File},
    io::AsyncReadExt,
    task::spawn_blocking,
};

use crate::{expire::Expire, msa_live::MSATokenResponce, request_token::XSTSToken};

#[derive(Debug)]
pub struct Cache {
    path: PathBuf,
    user_hash: String,
}

impl Cache {
    pub fn new(path: PathBuf, user_name: &str) -> Self {
        if !path.exists() {
            fs::create_dir(&path).unwrap();
        }
        Cache {
            path,
            user_hash: create_hash(user_name),
        }
    }
    pub async fn get_msa(&self) -> Result<Expire<MSATokenResponce>> {
        let path = self.path.join(format!("{}_msa-cache.json", self.user_hash));
        let mut buffer = vec![];
        File::open(path).await?.read_to_end(&mut buffer).await?;
        let ret = spawn_blocking(move || serde_json::from_slice(&buffer)).await??;
        Ok(ret)
    }
    pub async fn update_msa(&self, msa: &Expire<MSATokenResponce>) -> Result<()> {
        let path = self.path.join(format!("{}_msa-cache.json", self.user_hash));
        let content = serde_json::to_vec(msa)?;
        write(path, content).await?;
        Ok(())
    }

    pub async fn get_xsts(&self) -> Result<Expire<XSTSToken>> {
        let path = self.path.join(format!("{}_xbl-cache.json", self.user_hash));
        let mut buffer = vec![];
        File::open(path).await?.read_to_end(&mut buffer).await?;
        let ret = spawn_blocking(move || serde_json::from_slice(&buffer)).await??;
        Ok(ret)
    }
    pub async fn update_xsts(&self, xsts: &Expire<XSTSToken>) -> Result<()> {
        let path = self.path.join(format!("{}_xbl-cache.json", self.user_hash));
        let content = serde_json::to_vec(xsts)?;
        write(path, content).await?;
        Ok(())
    }
}

fn create_hash(user_name: &str) -> String {
    let mut sha256 = Sha256::new();
    sha256.update(user_name);
    sha256.finalize()[0..10]
        .iter()
        .map(|n| format!("{n:02x}"))
        .collect::<Vec<_>>()
        .join("")
}
