use std::{collections::HashMap, process::exit, sync::{Arc, mpsc::{Receiver, TryRecvError}}, time::Duration};

use futures_util::StreamExt;
use paho_mqtt::{
    properties, AsyncClient, ConnectOptionsBuilder, CreateOptionsBuilder, PropertyCode,
};
use relm4::ComponentSender;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, Value};
use tokio::sync::RwLock;

use crate::component::{
    connect::{ConnectDialog, Credentials},
    mqtt_worker::{AsyncHandler, AsyncHandlerMsg},
};

#[derive(Deserialize, Serialize, Clone)]
#[serde(untagged)]
enum InnerValue {
    String(String),
    Json(HashMap<String, Value>),
    JsonArray(Vec<HashMap<String, Value>>),
}

pub async unsafe fn mqtt_connection(
    connection_info: ConnectDialog,
    sender: ComponentSender<AsyncHandler>,
    rx: Receiver<()>
) {
    let data_map_lock: Arc<RwLock<HashMap<String, InnerValue>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let create_options = CreateOptionsBuilder::new()
        .server_uri(connection_info.url.clone())
        .client_id("")
        .finalize();

    let mut client = match AsyncClient::new(create_options) {
        Ok(client) => client,
        Err(error) => {
            sender.input(AsyncHandlerMsg::MessageReceived(format!(
                "Error when creating async client: {error}."
            )));
            exit(1)
        }
    };

    let mut stream = client.get_stream(300);

    let mut connect_options = ConnectOptionsBuilder::new();
    connect_options.keep_alive_interval(Duration::from_secs(30));
    connect_options.properties(properties![PropertyCode::SessionExpiryInterval => 3600]);
    connect_options.clean_session(true);

    if let Credentials::Yes(credentials) = connection_info.credentials {
        connect_options.user_name(credentials.username);
        connect_options.password(credentials.password);
    }

    let built_connect_options = connect_options.finalize();

    if let Err(error) = client.connect(built_connect_options).await {
        sender.input(AsyncHandlerMsg::MessageReceived(format!(
            "[ERROR] MQTT connection failed: {error}."
        )));
        exit(1)
    }

    match client
        .subscribe(
            connection_info
                .topic
                .clone()
                .unwrap_or_else(|| "#".to_string()),
            1,
        )
        .await
    {
        Err(error) => {
            sender.input(AsyncHandlerMsg::MessageReceived(format!(
                "[ERROR] Failed to subscribe topic: {error}."
            )));
            exit(1)
        }
        _ => {
            sender.input(AsyncHandlerMsg::MessageReceived(format!(
                "[INFO] Connected to {} with topic \"{}\".",
                connection_info.url,
                connection_info.topic.unwrap_or_else(|| "#".to_owned())
            )));
        }
    };
    let data_map_lock_cloned = data_map_lock.clone();

    while let Some(message_option) = stream.next().await {
        if let Some(message) = message_option {
            let topic = message.topic().to_owned();
            let payload = (*String::from_utf8_lossy(message.payload())).to_owned();
            sender.input(AsyncHandlerMsg::MessageReceived(format!(
                "[{}]\n{}",
                &topic, &payload
            )));
            if payload.starts_with('[') {
                if let Ok(deserialized_data) = from_str::<Vec<HashMap<String, Value>>>(&payload) {
                    data_map_lock_cloned
                        .write()
                        .await
                        .insert(topic, InnerValue::JsonArray(deserialized_data));
                } else {
                    data_map_lock_cloned
                        .write()
                        .await
                        .insert(topic, InnerValue::String(payload));
                };
            } else if let Ok(deserialized_data) = from_str::<HashMap<String, Value>>(&payload) {
                data_map_lock_cloned
                    .write()
                    .await
                    .insert(topic, InnerValue::Json(deserialized_data));
            } else {
                data_map_lock_cloned
                    .write()
                    .await
                    .insert(topic, InnerValue::String(payload));
            }
        } else {
            sender.input(AsyncHandlerMsg::MessageReceived(
                "[WARN] No message from the stream.".to_string(),
            ));
        }
        match rx.try_recv() {
            Ok(_) | Err(TryRecvError::Disconnected) => {
                break;
            }
            Err(TryRecvError::Empty) => {}
        }
    }
}
