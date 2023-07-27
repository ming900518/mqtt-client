use std::sync::mpsc::{Receiver, Sender};

use relm4::{ComponentSender, Worker};

use crate::{mqtt::mqtt_connection, Msg};

use super::connect::ConnectDialog;

#[derive(Debug)]
pub enum AsyncHandlerMsg {
    Start(ConnectDialog),
    Stop,
    MessageReceived(String),
}

pub struct AsyncHandler {
    tx: Option<Sender<()>>,
}

impl Worker for AsyncHandler {
    type Init = ();
    type Input = AsyncHandlerMsg;
    type Output = Msg;

    fn init(_init: Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { tx: None }
    }

    fn update(&mut self, msg: AsyncHandlerMsg, sender: ComponentSender<Self>) {
        match msg {
            AsyncHandlerMsg::Start(new_setting) => {
                let (tx, rx): (Sender<()>, Receiver<()>) = std::sync::mpsc::channel();
                self.tx = Some(tx);
                std::thread::spawn(|| {
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap()
                        .block_on(async {
                            unsafe {
                                mqtt_connection(new_setting, sender, rx).await;
                            }
                        });
                });
            }
            AsyncHandlerMsg::Stop => {
                if let Some(tx) = self.tx.clone() {
                    tx.send(()).ok();
                }
            }
            AsyncHandlerMsg::MessageReceived(message) => {
                sender.output(Msg::MessageReceived(message)).ok();
            }
        }
    }
}
