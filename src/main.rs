mod component;
mod mqtt;

use std::convert::identity;

use component::{
    about::AboutDialog,
    connect::ConnectDialog,
    mqtt_worker::{AsyncHandler, AsyncHandlerMsg},
};
use gtk::prelude::*;
use relm4::{
    adw, gtk, Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmApp,
    SimpleComponent, WorkerController,
};

struct App {
    about_dialog: Controller<AboutDialog>,
    connect_dialog: Controller<ConnectDialog>,
    worker: WorkerController<AsyncHandler>,
    text: String,
    started: bool,
}

#[derive(Debug)]
pub enum Msg {
    ShowAboutDialog,
    ShowConnectDialog,
    Start(ConnectDialog),
    Stop,
    MessageReceived(String),
}

#[relm4::component]
impl SimpleComponent for App {
    type Init = ();
    type Input = Msg;
    type Output = ();

    view! {
        #[root]
        adw::ApplicationWindow {
            set_vexpand: false,
            set_hexpand: false,
            set_overflow: gtk::Overflow::Hidden,
            #[name = "leaflet"]
            adw::Leaflet {
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,

                    #[name = "header"]
                    adw::HeaderBar {
                        #[name = "connect_button"]
                        pack_start = &gtk::Button {
                            set_icon_name: "list-add-symbolic",
                            connect_clicked[sender] => move |_| {
                            sender.input(Msg::Stop);
                                sender.input(Msg::ShowConnectDialog);
                            }
                        },

                        #[wrap(Some)]
                        set_title_widget = &adw::WindowTitle {
                            set_title: "MQTT Client",
                        },

                        #[name = "about_button"]
                        pack_end = &gtk::Button {
                            set_icon_name: "help-about-symbolic",
                            connect_clicked[sender] => move |_| {
                                sender.input(Msg::ShowAboutDialog);
                            }
                        },
                    },
                    gtk::ScrolledWindow {
                        #[wrap(Some)]
                        #[name = "textview"]
                        set_child = &gtk::TextView {
                            set_vexpand: true,
                            set_editable: false,
                            set_input_purpose: gtk::InputPurpose::Terminal,
                            set_overflow: gtk::Overflow::Visible,
                            set_monospace: true,
                            set_wrap_mode: gtk::WrapMode::Char,
                            #[wrap(Some)]
                            #[name = "buffer"]
                            set_buffer = &gtk::TextBuffer {
                                #[watch]
                                set_text: &model.text,
                            },
                        },
                        connect_vadjustment_notify => move |e| {
                            println!("connect_vadjustment_notify, {e:?}");
                        }
                    },
                },
            }
        }
    }

    fn init(
        _: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let about_dialog = AboutDialog::builder()
            .transient_for(root)
            .launch(())
            .detach();
        let connect_dialog = ConnectDialog::builder()
            .transient_for(root)
            .launch(())
            .forward(sender.input_sender(), identity);
        let model = App {
            about_dialog,
            connect_dialog,
            worker: AsyncHandler::builder()
                .detach_worker(())
                .forward(sender.input_sender(), identity),
            text: String::new(),
            started: false,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            Msg::ShowAboutDialog => self.about_dialog.sender().send(()).unwrap(),
            Msg::ShowConnectDialog => self.connect_dialog.widget().present(),
            Msg::Start(new_setting) => {
                self.worker.emit(AsyncHandlerMsg::Start(new_setting));
                self.started = true;
                self.text = String::new();
            }
            Msg::Stop => {
                self.worker.emit(AsyncHandlerMsg::Stop);
                self.started = false;
            }
            Msg::MessageReceived(message) => {
                self.text = if self.text.is_empty() {
                    message
                } else {
                    format!("{}\n\n{message}", self.text)
                };
            }
        }
    }
}

fn main() {
    let app = RelmApp::new("tw.mingchang.mqtt-client");
    app.run::<App>(());
}
