use gettextrs::gettext;
use tracing::{debug, info};

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, gio, glib};

use crate::config::{APP_ID, PKGDATADIR, PROFILE, VERSION};
use crate::window::ExampleApplicationWindow;
use adw::subclass::prelude::AdwApplicationImpl;

mod imp {
    use super::*;
    use glib::WeakRef;
    use std::cell::OnceCell;

    #[derive(Debug, Default)]
    pub struct ExampleApplication {
        pub window: OnceCell<WeakRef<ExampleApplicationWindow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ExampleApplication {
        const NAME: &'static str = "ExampleApplication";
        type Type = super::ExampleApplication;
        type ParentType = gtk::Application;
    }

    impl ObjectImpl for ExampleApplication {}

    impl ApplicationImpl for ExampleApplication {
        fn activate(&self) {
            debug!("GtkApplication<ExampleApplication>::activate");
            self.parent_activate();
            let app = self.obj();

            if let Some(window) = self.window.get() {
                let window = window.upgrade().unwrap();
                window.present();
                return;
            }

            let window = ExampleApplicationWindow::new(&app);
            self.window
                .set(window.downgrade())
                .expect("Window already set.");

            app.main_window().present();
        }

        fn startup(&self) {
            debug!("GtkApplication<ExampleApplication>::startup");
            self.parent_startup();
            let app = self.obj();

            // Set icons for shell
            gtk::Window::set_default_icon_name(APP_ID);

            app.setup_css();
            app.setup_accels();
        }
    }

    impl GtkApplicationImpl for ExampleApplication {}
    impl AdwApplicationImpl for ExampleApplication {}
}

glib::wrapper! {
    pub struct ExampleApplication(ObjectSubclass<imp::ExampleApplication>)
        @extends gio::Application, adw::Application,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl ExampleApplication {
    fn main_window(&self) -> ExampleApplicationWindow {
        self.imp().window.get().unwrap().upgrade().unwrap()
    }


    // Sets up keyboard shortcuts
    fn setup_accels(&self) {
        // self.set_accels_for_action("app.quit", &["<Control>q"]);
        // self.set_accels_for_action("window.close", &["<Control>w"]);
    }

    fn setup_css(&self) {
        let provider = gtk::CssProvider::new();
        provider.load_from_resource("/com/github/RoshanR/shamam/style.css");
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }


    pub fn run(&self) -> glib::ExitCode {
        info!("Shamam ({})", APP_ID);
        info!("Version: {} ({})", VERSION, PROFILE);
        info!("Datadir: {}", PKGDATADIR);

        ApplicationExtManual::run(self)
    }
}

impl Default for ExampleApplication {
    fn default() -> Self {
        glib::Object::builder()
            .property("application-id", APP_ID)
            .property("resource-base-path", "/com/github/RoshanR/shamam/")
            .build()
    }
}
