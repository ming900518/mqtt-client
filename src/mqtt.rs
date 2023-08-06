use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{atomic::Ordering, mpsc::Sender},
    time::Duration,
};

use paho_mqtt::{
    properties, AsyncClient, ConnectOptionsBuilder, CreateOptionsBuilder, PropertyCode,
};

use crate::{ConnectionInfo, Message, THREAD_STOP};

#[derive(Deserialize, Serialize, Clone)]
#[serde(untagged)]
enum InnerValue {
    String(String),
    Json(HashMap<String, Value>),
    JsonArray(Vec<HashMap<String, Value>>),
}

pub async fn connect_mqtt(tx: Sender<Message>, connection_info: ConnectionInfo) -> Result<(), ()> {
    let create_options = CreateOptionsBuilder::new()
        .server_uri(&connection_info.url)
        .client_id("")
        .finalize();

    let mut client = match AsyncClient::new(create_options) {
        Ok(client) => client,
        Err(error) => {
            tx.send(Message::Message(format!(
                "Error when creating async client: {}.",
                error
            )))
            .ok();
            return Err(());
        }
    };

    let mut stream = client.get_stream(300);

    let mut connect_options = ConnectOptionsBuilder::new();
    connect_options.keep_alive_interval(Duration::from_secs(30));
    connect_options.properties(properties![PropertyCode::SessionExpiryInterval => 3600]);
    connect_options.clean_session(true);

    if let Some(credential) = connection_info.credentials {
        connect_options.user_name(credential.username);
        connect_options.password(credential.password);
    }

    let built_connect_options = connect_options.finalize();

    if let Err(error) = client.connect(built_connect_options).await {
        tx.send(Message::Message(format!(
            "[ERROR] MQTT connection failed: {}.",
            error
        )))
        .ok();
        return Err(());
    }

    match client
        .subscribe(connection_info.topic.clone().unwrap_or("#".to_owned()), 1)
        .await
    {
        Err(error) => {
            tx.send(Message::Message(format!(
                "[ERROR] Failed to subscribe topic: {}.",
                error
            )))
            .ok();
            return Err(());
        }
        _ => {
            tx.send(Message::Message(format!(
                "[INFO] Connected to {} with topic \"{}\".",
                connection_info.url,
                connection_info.topic.clone().unwrap_or("#".to_owned())
            )))
            .ok();
        }
    };
    while let Some(message_option) = stream.next().await {
        if THREAD_STOP.load(Ordering::Relaxed) {
            tx.send(Message::Message("[INFO] THREAD_STOP received.".to_string()))
                .ok();
            break;
        }
        if let Some(mqtt_message) = message_option {
            let topic = mqtt_message.topic().to_owned();
            let payload = (*String::from_utf8_lossy(mqtt_message.payload())).to_owned();
            tx.send(Message::Message(format!("[{}]\n{}\n\n", &topic, &payload)))
                .ok();
        } else {
            tx.send(Message::Message(
                "[WARN] No message from the stream.".to_string(),
            ))
            .ok();
        }
    }
    Ok(())
}
