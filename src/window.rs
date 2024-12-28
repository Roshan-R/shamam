use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib::clone};

use gtk::glib;

use crate::application::ExampleApplication;
use crate::config::{APP_ID, PROFILE};
use std::sync::{mpsc, Arc};

use crate::core::http_thread::http_thread;
use crate::core::microphone_thread::microphone_thread;
use crate::core::processing_thread::processing_thread;
use crate::core::thread_messages::{GUIMessage, MicrophoneMessage};
use crate::utils::thread::spawn_big_thread;

use crate::gettext;
use crate::glib::MainLoop;
use adw::subclass::prelude::AdwApplicationWindowImpl;

mod imp {
    use crate::{main, utils};

    use super::*;

    #[derive(Debug, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/RoshanR/shamam/ui/window.ui")]
    pub struct ExampleApplicationWindow {
        #[template_child]
        pub welcome_page: TemplateChild<gtk::StackPage>,
        #[template_child]
        pub checking_page: TemplateChild<gtk::StackPage>,
        #[template_child]
        pub result_page: TemplateChild<gtk::StackPage>,

        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub button: TemplateChild<gtk::Button>,

        #[template_child]
        pub song_name: TemplateChild<gtk::Label>,
        #[template_child]
        pub song_desc: TemplateChild<gtk::Label>,
        #[template_child]
        pub result_image: TemplateChild<gtk::Image>,

        pub settings: gio::Settings,
    }

    impl Default for ExampleApplicationWindow {
        fn default() -> Self {
            Self {
                welcome_page: TemplateChild::default(),
                checking_page: TemplateChild::default(),
                result_page: TemplateChild::default(),

                song_name: TemplateChild::default(),
                song_desc: TemplateChild::default(),
                result_image: TemplateChild::default(),

                stack: TemplateChild::default(),
                button: TemplateChild::default(),
                settings: gio::Settings::new(APP_ID),
            }
        }
    }

    #[gtk::template_callbacks]
    impl ExampleApplicationWindow {
        #[template_callback]
        fn on_button_clicked(&self, button: &gtk::Button) {
            println!("Clicked me from a callback");
            self.stack.set_visible_child(&self.checking_page.child());
            gtk::glib::MainContext::default().acquire().unwrap();
            let main_loop = Arc::new(MainLoop::new(None, false));

            //let (gui_tx, gui_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
            let (gui_tx, gui_rx) = async_channel::unbounded();
            let (microphone_tx, microphone_rx) = mpsc::channel();
            let (processing_tx, processing_rx) = mpsc::channel();
            let (http_tx, http_rx) = mpsc::channel();

            let processing_microphone_tx = processing_tx.clone();
            let microphone_http_tx = microphone_tx.clone();

            spawn_big_thread(
                clone!(@strong gui_tx => move || { // microphone_rx, processing_tx
                    microphone_thread(microphone_rx, processing_microphone_tx, gui_tx);
                }),
            );

            spawn_big_thread(clone!(@strong gui_tx => move || { // processing_rx, http_tx
                processing_thread(processing_rx, http_tx, gui_tx);
            }));

            spawn_big_thread(clone!(@strong gui_tx => move || { // http_rx
                http_thread(http_rx, gui_tx, microphone_http_tx);
            }));

            let main_loop_cli = main_loop.clone();

            let self_weak = self.downgrade(); // Create a weak reference to `self`

            glib::MainContext::default().spawn_local(async move {
                if let Some(main_window) = self_weak.upgrade() {
                    while let Ok(msg) = gui_rx.recv().await {
                        match msg {
                            GUIMessage::DevicesList(device_names) => {
                                let dev_name = &device_names[0];
                                eprintln!("{} {}", gettext("Using device"), dev_name);
                                microphone_tx
                                    .send(MicrophoneMessage::MicrophoneRecordStart(
                                        dev_name.to_owned(),
                                    ))
                                    .unwrap();
                            }
                            GUIMessage::NetworkStatus(reachable) => {}
                            GUIMessage::ErrorMessage(string) => {
                                if string == gettext("No match for this song") {
                                    eprintln!("{} {}", gettext("Error:"), string);
                                }
                            }
                            GUIMessage::MicrophoneRecording => {
                                eprintln!("{}", gettext("Recording started!"));
                            }
                            GUIMessage::SongRecognized(message) => {
                                // let mut last_track_borrow = last_track.borrow_mut();
                                let track_key = Some(message.track_key.clone());
                                let song_name =
                                    format!("{} - {}", message.artist_name, message.song_name);
                                println!("{} {}", song_name, track_key.unwrap());
                                dbg!(message.shazam_json);

                                main_window.song_name.set_text(&song_name);
                                main_window.song_desc.set_text(&song_name);
                                let loader = gdk_pixbuf::PixbufLoader::new();
                                loader.write(&message.cover_image.unwrap()).unwrap();
                                loader.close().unwrap();
                                let pixbuf = loader.pixbuf();

                                main_window.result_image.set_from_pixbuf(pixbuf.as_ref());

                                main_window
                                    .stack
                                    .set_visible_child(&main_window.result_page.child());

                                main_loop_cli.quit();
                            }
                            _ => {}
                        }
                    }
                }
            });

            // let number_increased = self.number.get() + 1;
            // self.number.set(number_increased);
            // button.set_label(&number_increased.to_string())
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ExampleApplicationWindow {
        const NAME: &'static str = "ExampleApplicationWindow";
        type Type = super::ExampleApplicationWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ExampleApplicationWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            // Devel Profile
            if PROFILE == "Devel" {
                obj.add_css_class("devel");
            }

            // Load latest window state
            obj.load_window_size();
            self.stack.set_transition_duration(3000);
        }
    }

    impl WidgetImpl for ExampleApplicationWindow {}
    impl WindowImpl for ExampleApplicationWindow {
        // Save window state on delete event
        fn close_request(&self) -> glib::Propagation {
            if let Err(err) = self.obj().save_window_size() {
                tracing::warn!("Failed to save window state, {}", &err);
            }

            // Pass close request on to the parent
            self.parent_close_request()
        }
    }

    impl ApplicationWindowImpl for ExampleApplicationWindow {}
    impl AdwApplicationWindowImpl for ExampleApplicationWindow {}
}

glib::wrapper! {
    pub struct ExampleApplicationWindow(ObjectSubclass<imp::ExampleApplicationWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Root;
}

impl ExampleApplicationWindow {
    pub fn new(app: &ExampleApplication) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let imp = self.imp();

        let (width, height) = self.default_size();

        imp.settings.set_int("window-width", width)?;
        imp.settings.set_int("window-height", height)?;

        imp.settings
            .set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let imp = self.imp();

        let width = imp.settings.int("window-width");
        let height = imp.settings.int("window-height");
        let is_maximized = imp.settings.boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
    }

    fn show_result(&mut self, song_name: String, song_desc: String) {}
}
