use gtk::prelude::GtkWindowExt;
use relm4::{adw, gtk, ComponentParts, ComponentSender, SimpleComponent};

pub struct AboutDialog {}

impl SimpleComponent for AboutDialog {
    type Init = ();
    type Widgets = adw::AboutWindow;
    type Input = ();
    type Output = ();
    type Root = adw::AboutWindow;

    fn init_root() -> Self::Root {
        adw::AboutWindow::builder()
            .application_icon("application-x-executable-symbolic")
            .application_name("MQTT Client")
            .comments("A MQTT Client with GTK4 GUI support.")
            .website("https://mingchang.tw")
            .version("0.2.0")
            .copyright("© 2023 Ming Chang")
            .developers(vec![String::from("Ming Chang")])
            .designers(vec![String::from("Ming Chang")])
            .modal(true)
            .hide_on_close(true)
            .build()
    }

    fn init(
        _: Self::Init,
        root: &Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {};

        let widgets = root.clone();

        ComponentParts { model, widgets }
    }

    fn update_view(&self, dialog: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        dialog.present();
    }
}
