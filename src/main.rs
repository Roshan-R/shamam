mod application;
#[rustfmt::skip]
mod config;

mod fingerprinting {
    pub mod algorithm;
    pub mod communication;
    mod hanning;
    pub mod signature_format;
    mod user_agent;
}
mod core {
    pub mod http_thread;
    pub mod microphone_thread;
    pub mod processing_thread;
    pub mod thread_messages;
}

mod utils {
    pub mod ffmpeg_wrapper;
    pub mod internationalization;
    pub mod mpris_player;
    pub mod thread;
}

mod window;

use gettextrs::{gettext, LocaleCategory};
use gtk::{gio, glib};

use self::application::ExampleApplication;
use self::config::{GETTEXT_PACKAGE, LOCALEDIR, RESOURCES_FILE};

fn main() -> glib::ExitCode {
    // Initialize logger
    tracing_subscriber::fmt::init();

    // Prepare i18n
    gettextrs::setlocale(LocaleCategory::LcAll, "");
    gettextrs::bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR);
    gettextrs::textdomain(GETTEXT_PACKAGE);

    glib::set_application_name(&gettext("Shamam"));

    let res = gio::Resource::load(RESOURCES_FILE).expect("Could not load gresource file");
    gio::resources_register(&res);

    let app = ExampleApplication::default();
    app.run()
}
