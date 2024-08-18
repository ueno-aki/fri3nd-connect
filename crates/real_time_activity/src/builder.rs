use std::sync::Arc;

use anyhow::Result;
use futures_util::StreamExt;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::ClientRequestBuilder};
use xbl_auth::XBLAuth;

use crate::{RtaClient, WSWriter};

pub struct RtaClientBuilder {
    xbl_auth: Arc<Mutex<XBLAuth>>,
    uri: String,
    ev_bounds: usize,
    subscription_urls: Vec<String>,
}

impl RtaClientBuilder {
    pub fn new(xbl_auth: Arc<Mutex<XBLAuth>>) -> Self {
        Self {
            xbl_auth,
            uri: "".to_owned(),
            subscription_urls: vec![],
            ev_bounds: 32,
        }
    }

    pub fn set_uri(mut self, uri: String) -> Self {
        self.uri = uri;
        self
    }

    pub fn set_ev_bounds(mut self, buffer: usize) -> Self {
        self.ev_bounds = buffer;
        self
    }

    pub fn add_subscription(mut self, uri: String) -> Self {
        self.subscription_urls.push(uri);
        self
    }

    pub async fn connect(self) -> Result<RtaClient> {
        let Self {
            xbl_auth,
            uri,
            subscription_urls,
            ev_bounds,
        } = self;
        let (rta_writer, rta_reader) = mpsc::channel(ev_bounds);
        let authorization = {
            let xsts = xbl_auth.lock().await.get_xbox_token().await?.take();
            format!("XBL3.0 x={};{}", xsts.user_hash, xsts.token)
        };
        let builder = ClientRequestBuilder::new(uri.parse()?)
            .with_header("authorization", &authorization)
            .with_sub_protocol("rta.xboxlive.com.V2");
        let (socket, _) = connect_async(builder).await?;
        let (ws_writer, ws_reader) = socket.split();
        println!("Open RTA connection");
        Ok(RtaClient {
            subscription_urls,
            ws_writer: WSWriter::new(Arc::new(Mutex::new(ws_writer))),
            ws_reader,
            rta_writer,
            rta_reader,
        })
    }
}
