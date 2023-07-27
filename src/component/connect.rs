use relm4::{
    adw::{self, prelude::*},
    gtk, ComponentParts, ComponentSender, SimpleComponent,
};

use crate::Msg;

#[derive(Debug, Default, Clone)]
pub struct ConnectDialog {
    pub url: String,
    pub topic: Option<String>,
    pub credentials: Credentials,
}

#[derive(Debug, Default, Clone)]
pub enum Credentials {
    #[default]
    No,
    Yes(ProvidedCredentials),
}

#[derive(Debug, Clone)]
pub struct ProvidedCredentials {
    pub username: String,
    pub password: String,
}

#[relm4::component(pub)]
impl SimpleComponent for ConnectDialog {
    type Init = ();
    type Input = ConnectDialog;
    type Output = Msg;

    view! {
        #[name = "connect_dialog"]
        gtk::Window {
            set_default_width: 480,
            set_modal: true,
            set_hide_on_close: true,
            #[wrap(Some)]
            set_titlebar = &adw::HeaderBar {
                set_show_start_title_buttons: false,
                set_show_end_title_buttons: false,
                pack_start = &gtk::Button {
                    set_label: "Cancel",
                    connect_clicked[connect_dialog] => move |_| {
                        connect_dialog.close();
                    }
                },

                #[wrap(Some)]
                set_title_widget = &adw::WindowTitle {
                    set_title: "Connect to MQTT Server",
                },

                #[name = "connect_dialog_connect_btn"]
                pack_end = &gtk::Button {
                    set_label: "Connect",
                    set_sensitive: false,
                    connect_clicked[connect_dialog, connect_dialog_url, connect_dialog_topic, connect_dialog_username, connect_dialog_password, sender] => move |_| {
                        let url = connect_dialog_url.text().to_string();
                        let topic = connect_dialog_topic.text().to_string();
                        let username = connect_dialog_username.text().to_string();
                        let password = connect_dialog_password.text().to_string();
                        let new_setting = ConnectDialog {
                            url,
                            topic: if !topic.is_empty() { Some(topic) } else { None },
                            credentials:
                                if let (true, true) = (!username.is_empty(), !password.is_empty()) {
                                    Credentials::Yes(
                                        ProvidedCredentials {
                                            username,
                                            password
                                        }
                                    )
                                } else {
                                    Credentials::No
                                }
                        };
                        if let Err(e) = sender.output(Msg::Start(new_setting)) {
                            println!("{e:?}");
                        } else {
                            connect_dialog.close();
                        }
                    }
                },
            },
            adw::PreferencesPage {
                add = &adw::PreferencesGroup {
                    set_title: "Connection Info",
                    #[name = "connect_dialog_url"]
                    adw::EntryRow {
                        set_title: "URL",
                        set_input_purpose: gtk::InputPurpose::Url,
                        connect_changed[connect_dialog_connect_btn] => move |e| {
                            connect_dialog_connect_btn.set_sensitive(!e.text().to_string().is_empty());
                        }
                    },
                    #[name = "connect_dialog_topic"]
                    adw::EntryRow {
                        set_title: "Topic (Optional)"
                    },
                },
                add = &adw::PreferencesGroup {
                    set_title: "Credentials",
                    set_description: Some("Optional"),
                    #[wrap(Some)]
                     set_header_suffix = &gtk::CheckButton {
                            set_label: Some("Use Credentials"),
                            connect_toggled[connect_dialog_username, connect_dialog_password] => move |_| {
                                connect_dialog_username.set_sensitive(!connect_dialog_username.is_sensitive());
                                connect_dialog_username.set_text("");
                                connect_dialog_password.set_sensitive(!connect_dialog_password.is_sensitive());
                                connect_dialog_password.set_text("");
                            }
                        },
                    #[name = "connect_dialog_username"]
                    adw::EntryRow {
                        set_title: "Username",
                        set_sensitive: false,
                    },
                    #[name = "connect_dialog_password"]
                    adw::PasswordEntryRow {
                        set_title: "Password",
                        set_input_purpose: gtk::InputPurpose::Password,
                        set_sensitive: false,
                    },
                }
            }
        }
    }

    fn init(
        _: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            url: String::new(),
            topic: None,
            credentials: Credentials::No,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}
