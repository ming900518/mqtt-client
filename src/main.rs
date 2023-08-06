mod mqtt;

use crate::mqtt::connect_mqtt;
use std::{
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    thread::JoinHandle,
    time::Duration,
};

use gtk4::{
    gdk::Display, glib::ControlFlow, prelude::*, style_context_add_provider_for_display, Align,
    Box, CssProvider, InputPurpose, Orientation, ScrolledWindow, Switch, TextBuffer, TextView,
    ToggleButton, WrapMode, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use leptos::*;
use libadwaita::{
    traits::{AdwApplicationWindowExt, AdwWindowExt, PreferencesGroupExt},
    AboutWindow, Application, ApplicationWindow, EntryRow, HeaderBar, PasswordEntryRow,
    PreferencesGroup, PreferencesWindow,
};

const APP_ID: &str = "tw.mingchang.mqtt-client";

#[derive(Debug, Default, Clone)]
pub struct ConnectionInfo {
    pub url: String,
    pub topic: Option<String>,
    pub credentials: Option<ProvidedCredentials>,
}

#[derive(Debug, Clone, Default)]
pub struct ProvidedCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    Message(String),
    Stop,
}

static THREAD: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);
static THREAD_STOP: AtomicBool = AtomicBool::new(false);
fn main() {
    _ = create_scope(create_runtime(), |cx| {
        let app = Application::builder().application_id(APP_ID).build();
        app.connect_startup(|_| {
            let provider = CssProvider::new();
            provider.load_from_data(include_str!("style.css"));

            // Add the provider to the default screen
            style_context_add_provider_for_display(
                &Display::default().expect("Could not connect to a display."),
                &provider,
                STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        });
        app.connect_activate(move |app| {
            let connection_info = create_rw_signal(cx, ConnectionInfo::default());
            let connection = create_rw_signal(cx, false);

            let window = Rc::new(
                ApplicationWindow::builder()
                    .title("MQTT Client")
                    .application(app)
                    .default_width(1000)
                    .default_height(600)
                    .build(),
            );

            let text_buffer = TextBuffer::default();

            let content = Box::new(Orientation::Vertical, 0);
            content.append(&header_bar(
                cx,
                app,
                &window,
                connection_info,
                connection,
                text_buffer.clone(),
            ));
            content.append(&main_ui(cx, app, text_buffer));
            window.set_content(Some(&content));
            window.present();
        });
        app.run();
    });
}

fn header_bar(
    cx: Scope,
    _app: &Application,
    window: &ApplicationWindow,
    connection_info: RwSignal<ConnectionInfo>,
    connection: RwSignal<bool>,
    text_buffer: TextBuffer,
) -> HeaderBar {
    let header_bar = HeaderBar::new();

    let add_button = ToggleButton::new();
    add_button.connect_clicked({
        let window = window.clone();
        let text_buffer = text_buffer.clone();
        move |e| {
            e.set_active(false);
            if connection.get_untracked() {
                THREAD_STOP.store(true, Ordering::Relaxed);
                if let Ok(mut handle_option) = THREAD.try_lock() {
                    loop {
                        if handle_option
                            .as_ref()
                            .map_or_else(|| false, |handle| handle.is_finished())
                        {
                            connection.set(false);
                            connection_info.set(ConnectionInfo::default());
                            *handle_option = None;
                            break;
                        }
                    }
                }
            } else {
                connect_window(
                    cx,
                    &window,
                    connection_info,
                    connection,
                    text_buffer.clone(),
                )
                .present();
            }
        }
    });

    create_effect(cx, {
        let add_button = add_button.clone();
        move |_| {
            if connection.get() {
                add_button.set_icon_name("media-playback-stop");
            } else {
                add_button.set_icon_name("list-add-symbolic");
            }
        }
    });

    let about_button = ToggleButton::new();
    about_button.set_icon_name("help-about-symbolic");
    about_button.connect_clicked({
        let window = window.clone();
        move |e| {
            e.set_active(false);
            about_window(&window).present()
        }
    });

    header_bar.pack_start(&add_button);
    header_bar.pack_end(&about_button);
    header_bar
}

fn main_ui(_cx: Scope, _app: &Application, text_buffer: TextBuffer) -> Box {
    let main_box = Box::builder().vexpand(true).hexpand(true).build();

    let scrolled_view = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .build();
    let text_view = TextView::builder()
        .monospace(true)
        .wrap_mode(WrapMode::Char)
        .overwrite(false)
        .buffer(&text_buffer)
        .build();
    scrolled_view.set_child(Some(&text_view));

    main_box.append(&scrolled_view);
    main_box
}

fn about_window(parent: &ApplicationWindow) -> AboutWindow {
    AboutWindow::builder()
        .application_icon("application-x-executable-symbolic")
        .application_name("MQTT Client")
        .comments("A MQTT Client with GTK4 GUI support.")
        .website("https://mingchang.tw")
        .version("0.3.0")
        .copyright("© 2023 Ming Chang")
        .developers(vec![String::from("Ming Chang")])
        .designers(vec![String::from("Ming Chang")])
        .modal(true)
        .transient_for(parent)
        .build()
}

fn connect_window(
    cx: Scope,
    parent: &ApplicationWindow,
    connection_info: RwSignal<ConnectionInfo>,
    connection: RwSignal<bool>,
    text_buffer: TextBuffer,
) -> PreferencesWindow {
    let window = PreferencesWindow::builder()
        .title("Connect to MQTT Server")
        .default_width(400)
        .default_height(500)
        .transient_for(parent)
        .modal(true)
        .build();

    let window_content = Box::new(Orientation::Vertical, 0);
    let content = Box::builder()
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(20)
        .margin_end(20)
        .spacing(20)
        .orientation(Orientation::Vertical)
        .build();

    let header_bar = HeaderBar::new();

    let cancel_button = ToggleButton::builder().label("Cancel").build();
    cancel_button.connect_clicked({
        let window = window.clone();
        move |_| window.destroy()
    });
    let submit_button = ToggleButton::builder().label("Connect").build();
    submit_button.connect_clicked({
        let window = window.clone();
        move |_| {
            let (tx, rx) = std::sync::mpsc::channel::<Message>();
            let connection_info = connection_info.get_untracked();
            if let Ok(mut writer) = THREAD.try_lock() {
                writer.replace(std::thread::spawn(move || {
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap()
                        .block_on(async {
                            connect_mqtt(tx, connection_info).await.ok();
                        });
                }));
            }
            let text_buffer = text_buffer.clone();
            gtk4::glib::timeout_add_local(Duration::from_millis(50), move || {
                if connection.get_untracked() {
                    if let Ok(Message::Message(mqtt_message)) = rx.recv() {
                        text_buffer.insert(&mut text_buffer.iter_at_offset(0), &mqtt_message);
                    }
                    ControlFlow::Continue
                } else {
                    ControlFlow::Break
                }
            });
            connection.set(true);
            window.destroy();
        }
    });

    header_bar.pack_start(&cancel_button);
    header_bar.pack_end(&submit_button);

    window_content.append(&header_bar);

    let info_preferences_group = PreferencesGroup::builder().title("Connection Info").build();
    let connect_dialog_url = EntryRow::builder()
        .title("URL")
        .input_purpose(InputPurpose::Url)
        .build();
    connect_dialog_url
        .connect_changed(move |e| connection_info.update(|old| old.url = e.text().to_string()));
    let connect_dialog_topic = EntryRow::builder().title("Topic (Optional)").build();
    connect_dialog_topic.connect_changed(move |e| {
        connection_info.update(|old| {
            old.topic = if !e.text().is_empty() {
                Some(e.text().to_string())
            } else {
                None
            }
        })
    });
    info_preferences_group.add(&connect_dialog_url);
    info_preferences_group.add(&connect_dialog_topic);
    content.append(&info_preferences_group);

    let credential_preferences_group = PreferencesGroup::builder()
        .title("Credentials")
        .description("Optional")
        .build();

    let credential_state = create_rw_signal(cx, false);
    let credential_toggle = Switch::builder().state(false).valign(Align::Center).build();
    credential_toggle.connect_state_notify(move |e| credential_state.set(e.state()));
    credential_preferences_group.set_header_suffix(Some(&credential_toggle));

    let connect_dialog_username = EntryRow::builder()
        .title("Username")
        .sensitive(false)
        .build();
    connect_dialog_username.connect_changed(move |e| {
        connection_info.update(|old| {
            let new_credentials = ProvidedCredentials {
                username: e.text().to_string(),
                password: old
                    .credentials
                    .clone()
                    .map_or_else(String::default, |credential| credential.password),
            };
            old.credentials = Some(new_credentials);
        })
    });
    credential_preferences_group.add(&connect_dialog_username);

    let connect_dialog_password = PasswordEntryRow::builder()
        .title("Password")
        .sensitive(false)
        .input_purpose(InputPurpose::Password)
        .build();
    connect_dialog_password.connect_changed(move |e| {
        connection_info.update(|old| {
            let new_credentials = ProvidedCredentials {
                username: old
                    .credentials
                    .clone()
                    .map_or_else(String::default, |credential| credential.username),
                password: e.text().to_string(),
            };
            old.credentials = Some(new_credentials);
        })
    });
    credential_preferences_group.add(&connect_dialog_password);

    create_effect(cx, {
        let connect_dialog_username = connect_dialog_username.clone();
        let connect_dialog_password = connect_dialog_password.clone();
        move |_| {
            let state = credential_state.get();
            if !state {
                connect_dialog_username.set_text("");
                connect_dialog_password.set_text("");
                connection_info.update(|old| old.credentials = None);
            }
            connect_dialog_username.set_sensitive(state);
            connect_dialog_password.set_sensitive(state);
        }
    });

    content.append(&credential_preferences_group);

    window_content.append(&content);

    window.set_content(Some(&window_content));
    window
}
