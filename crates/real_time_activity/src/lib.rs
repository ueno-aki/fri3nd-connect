use serde::{Deserialize, Serialize};

use std::path::Path;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{ClientRequestBuilder, Message},
};
use xbl_auth::XBLAuth;

use crate::message::MessageData;
pub mod message;
pub mod status;

#[derive(Debug, Default)]
pub struct RtaClient {
    sequence_id: u64,
}
impl RtaClient {
    const WS_ADDRESS: &'static str = "wss://rta.xboxlive.com/connect";

    pub fn new() -> Self {
        RtaClient::default()
    }

    pub async fn init(&self) -> Result<()> {
        let xbl_auth = XBLAuth::new(Path::new("../../auth"), "Ferris");
        println!("xbl_authed");

        let authorization = {
            let xsts = xbl_auth.get_xbox_token().await?.take();
            format!("XBL3.0 x={};{}", xsts.user_hash, xsts.token)
        };
        let builder = ClientRequestBuilder::new(Self::WS_ADDRESS.parse()?)
            .with_header("authorization", &authorization)
            .with_sub_protocol("rta.xboxlive.com.V2");
        let (mut socket, _) = connect_async(builder).await?;
        println!("Open RTA connection");
        socket
            .send(Message::Text(
                r#"[1,1,"https://sessiondirectory.xboxlive.com/connections/"]"#.into(),
            ))
            .await?;
        while let Some(Ok(msg)) = socket.next().await {
            pub use tokio_tungstenite::tungstenite::Message::*;
            match msg {
                Text(ref v) => {
                    let v: MessageData<ConnectionId> = serde_json::from_str(v)?;
                    println!("Text:{v:?}");
                }
                Binary(v) => {
                    println!("Binary:{v:?}");
                }
                Close(v) => {
                    println!("Close:{v:?}");
                }
                _ => break,
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConnectionId {
    connection_id: String,
}

#[cfg(test)]
mod tests {

    use anyhow::Result;

    use crate::RtaClient;

    #[tokio::test]
    async fn it_works() -> Result<()> {
        let client = RtaClient::new();
        client.init().await?;
        Ok(())
    }
}
