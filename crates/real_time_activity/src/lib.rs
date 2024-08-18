use std::{sync::Arc, time::Duration};

use event::RtaEvent;
use message::{MessageData, MessageType};

use anyhow::{bail, Result};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use status::Status;
use tokio::{
    net::TcpStream,
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

pub mod builder;
pub mod event;
pub mod message;
pub mod status;

#[derive(Debug)]
pub struct RtaClient {
    subscription_urls: Vec<String>,
    ws_writer: WSWriter,
    ws_reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    rta_writer: Sender<RtaEvent>,
    rta_reader: Receiver<RtaEvent>,
}

impl RtaClient {
    pub fn listen(self) -> Result<(WSWriter, Receiver<RtaEvent>)> {
        let Self {
            subscription_urls,
            ws_writer,
            ws_reader,
            rta_writer,
            rta_reader,
        } = self;
        let ws_writer_c = ws_writer.clone();
        let mut stream = RtaStream::new(subscription_urls, ws_writer, ws_reader, rta_writer);
        tokio::spawn(async move {
            if let Err(e) = stream.run().await {
                println!("RtaClientError: \"{e:?}\".");
            }
        });
        Ok((ws_writer_c, rta_reader))
    }
}

#[derive(Debug, Clone)]
pub struct WSWriter {
    sequence_id: i64,
    writer: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
}
impl WSWriter {
    pub fn new(
        writer: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
    ) -> Self {
        Self {
            sequence_id: 0,
            writer,
        }
    }

    pub async fn subscribe(&mut self, uri: &str) -> Result<()> {
        self.send(Message::Text(format!(
            r#"[{},{},"{}"]"#,
            MessageType::Subscribe as u8,
            self.sequence_id,
            uri
        )))
        .await?;
        Ok(())
    }
    pub async fn unsubscribe(&mut self, subscription_id: i64) -> Result<()> {
        self.send(Message::Text(format!(
            r#"[{},{},{}]"#,
            MessageType::Subscribe as u8,
            self.sequence_id,
            subscription_id
        )))
        .await?;
        Ok(())
    }

    #[inline]
    pub async fn close(&mut self) -> Result<()> {
        self.send(Message::Close(None)).await?;
        Ok(())
    }
    #[inline]
    pub async fn send(&mut self, message: Message) -> Result<()> {
        self.sequence_id += 1;
        self.writer.lock().await.send(message).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct RtaStream {
    pre_subscription_urls: Vec<String>,
    ws_writer: WSWriter,
    ws_reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    rta_writer: Sender<RtaEvent>,
}
impl RtaStream {
    pub fn new(
        pre_sub_urls: Vec<String>,
        ws_writer: WSWriter,
        ws_reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        rta_writer: Sender<RtaEvent>,
    ) -> Self {
        Self {
            pre_subscription_urls: pre_sub_urls,
            ws_writer,
            ws_reader,
            rta_writer,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        for sub in self.pre_subscription_urls.iter() {
            self.ws_writer.subscribe(sub).await?;
        }
        loop {
            match tokio::time::timeout(Duration::from_millis(30000), self.ws_reader.next()).await {
                Ok(Some(Ok(msg))) => match msg {
                    Message::Text(v) => match serde_json::from_str(&v)? {
                        MessageData::Subscribe {
                            seq_id,
                            status,
                            sub_id,
                            connection_id,
                        } => {
                            if status == Status::Success {
                                self.rta_writer
                                    .send(RtaEvent::Subscribe {
                                        seq_id,
                                        sub_id,
                                        connection_id,
                                    })
                                    .await?;
                            } else {
                                bail!("Subscribe failed: {status:?}");
                            }
                        }
                        MessageData::Unsubscribe { seq_id, status } => {
                            if status == Status::Success {
                                self.rta_writer
                                    .send(RtaEvent::Unsubscribe { seq_id })
                                    .await?;
                            } else {
                                bail!("Unsubscribe failed: {status:?}");
                            }
                        }
                        MessageData::Event { sub_id, data } => {
                            self.rta_writer
                                .send(RtaEvent::Event { sub_id, data })
                                .await?;
                        }
                        MessageData::Resync => {}
                    },
                    Message::Close(v) => {
                        println!("Close:{v:?}");
                    }
                    Message::Pong(v) => {
                        self.rta_writer.send(RtaEvent::Pong(v)).await?;
                    }
                    _ => {}
                },
                Ok(Some(Err(e))) => {
                    println!("WebSocketErrorOccured: {e:?}.");
                    break;
                }
                Ok(None) => {
                    println!("WebSocketDisconnected.");
                    break;
                }
                Err(..) => {
                    println!("Timeout: no response in 30000 milliseconds.");
                    self.ws_writer.send(Message::Close(None)).await?;
                    break;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;
    use tokio::sync::Mutex;
    use xbl_auth::XBLAuth;

    use crate::builder::RtaClientBuilder;

    #[tokio::test]
    async fn it_works() -> Result<()> {
        let xbl_auth = Arc::new(Mutex::new(XBLAuth::new(
            "../../auth".parse()?,
            "Ferris".into(),
        )));
        println!("xbl_authed");
        let client = RtaClientBuilder::new(xbl_auth.clone())
            .set_uri("wss://rta.xboxlive.com/connect".to_owned())
            .set_ev_bounds(32)
            .add_subscription("https://sessiondirectory.xboxlive.com/connections/".to_owned())
            .connect()
            .await?;
        let (mut ws_writer, mut rx) = client.listen()?;
        while let Some(v) = rx.recv().await {
            println!("{v:?}");
            ws_writer.close().await?;
        }
        Ok(())
    }
}
