use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, Value};
use std::{
    collections::HashMap,
    process::exit,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
use tokio::sync::RwLock;

use paho_mqtt::{
    properties, AsyncClient, ConnectOptionsBuilder, CreateOptionsBuilder, PropertyCode,
};

use crate::{ConnectionInfo, THREAD_STOP};

#[derive(Deserialize, Serialize, Clone)]
#[serde(untagged)]
enum InnerValue {
    String(String),
    Json(HashMap<String, Value>),
    JsonArray(Vec<HashMap<String, Value>>),
}

pub async fn connect_mqtt(connection_info: ConnectionInfo) {
    let data_map_lock: Arc<RwLock<HashMap<String, InnerValue>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let create_options = CreateOptionsBuilder::new()
        .server_uri(&connection_info.url)
        .client_id("")
        .finalize();

    let mut client = match AsyncClient::new(create_options) {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Error when creating async client: {}.", error);
            exit(1)
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
        eprintln!("[ERROR] MQTT connection failed: {}.", error);
        exit(1)
    }

    match client
        .subscribe(connection_info.topic.clone().unwrap_or("#".to_owned()), 1)
        .await
    {
        Err(error) => {
            eprintln!("[ERROR] Failed to subscribe topic: {}.", error);
            exit(1)
        }
        _ => eprintln!(
            "[INFO] Connected to {} with topic \"{}\".",
            connection_info.url,
            connection_info.topic.unwrap_or("#".to_owned())
        ),
    };
    let data_map_lock_cloned = data_map_lock.clone();

    while let Some(message_option) = stream.next().await {
        if THREAD_STOP.load(Ordering::Relaxed) {
            eprintln!("[INFO] THREAD_STOP received.");
            break;
        }
        if let Some(message) = message_option {
            let topic = message.topic().to_owned();
            let payload = (*String::from_utf8_lossy(message.payload())).to_owned();
            eprintln!("[{}]\n{}\n\n", &topic, &payload);
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
            eprintln!("[WARN] No message from the stream.");
        }
    }
}
