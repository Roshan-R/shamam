use std::error::Error;
use std::sync::{mpsc, Arc};

use gettextrs::gettext;
use glib;
use glib::clone;

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

mod cli {
    pub mod cli_main;
}

mod utils {
    pub mod ffmpeg_wrapper;
    pub mod internationalization;
    pub mod mpris_player;
    pub mod thread;
}

pub enum CLIOutputType {
    SongName,
    JSON,
    CSV,
}

use crate::core::http_thread::http_thread;
use crate::core::microphone_thread::microphone_thread;
use crate::core::processing_thread::processing_thread;
use crate::core::thread_messages::{GUIMessage, MicrophoneMessage};

use crate::utils::thread::spawn_big_thread;
pub fn main() -> Result<(), Box<dyn Error>> {
    glib::MainContext::default().acquire();
    let main_loop = Arc::new(glib::MainLoop::new(None, false));

    let (gui_tx, gui_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
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

    gui_rx.attach(None, move |gui_message| {
        match gui_message {
            GUIMessage::DevicesList(device_names) => {
                // no need to start a microphone if recognizing from file
                // if input_file_name.is_some() {
                //     return glib::Continue(true);
                // }
                // let dev_name = if let Some(dev) = &audio_dev_name {
                //     if !device_names.contains(dev) {
                //         eprintln!("{}", gettext("Exiting: audio device not found"));
                //         main_loop_cli.quit();
                //         return glib::Continue(false);
                //     }
                //     dev
                // } else {
                //     if device_names.is_empty() {
                //         eprintln!("{}", gettext("Exiting: no audio devices found!"));
                //         main_loop_cli.quit();
                //         return glib::Continue(false);
                //     }
                //     &device_names[0]
                // };
                let dev_name = &device_names[0];
                eprintln!("{} {}", gettext("Using device"), dev_name);
                microphone_tx
                    .send(MicrophoneMessage::MicrophoneRecordStart(
                        dev_name.to_owned(),
                    ))
                    .unwrap();
            }
            GUIMessage::NetworkStatus(reachable) => {
                // let mpris_status = if reachable {
                //     PlaybackStatus::Playing
                // } else {
                //     PlaybackStatus::Paused
                // };
                // mpris_player
                //     .as_ref()
                //     .map(|p| p.set_playback_status(mpris_status));
                //
                // if !reachable {
                //     if input_file_name.is_some() {
                //         eprintln!("{}", gettext("Error: Network unreachable"));
                //         main_loop_cli.quit();
                //         return glib::Continue(false);
                //     } else {
                //         eprintln!("{}", gettext("Warning: Network unreachable"));
                //     }
                // }
            }
            GUIMessage::ErrorMessage(string) => {
                //if !(string == gettext("No match for this song") && !input_file_name.is_some()) {
                if string == gettext("No match for this song") {
                    eprintln!("{} {}", gettext("Error:"), string);
                }
                // if input_file_name.is_some() {
                //     main_loop_cli.quit();
                //     return glib::Continue(false);
                // }
            }
            GUIMessage::MicrophoneRecording => {
                eprintln!("{}", gettext("Recording started!"));
                // if !do_recognize_once {
                //     eprintln!("{}", gettext("Recording started!"));
                // }
            }
            GUIMessage::SongRecognized(message) => {
                // let mut last_track_borrow = last_track.borrow_mut();
                let track_key = Some(message.track_key.clone());
                let song_name = format!("{} - {}", message.artist_name, message.song_name);

                println!("{} {}", song_name, track_key.unwrap());

                dbg!(message.shazam_json);

                // if *last_track_borrow != track_key {
                //     mpris_player.as_ref().map(|p| update_song(p, &message));
                //     *last_track_borrow = track_key;
                //     match parameters.output_type {
                //         CLIOutputType::JSON => {
                //             println!("{}", message.shazam_json);
                //         }
                //         CLIOutputType::CSV => {
                //             csv_writer
                //                 .serialize(SongHistoryRecord {
                //                     song_name: song_name,
                //                     album: message
                //                         .album_name
                //                         .as_ref()
                //                         .unwrap_or(&"".to_string())
                //                         .to_string(),
                //                     recognition_date: Local::now().format("%c").to_string(),
                //                     track_key: message.track_key,
                //                     release_year: message
                //                         .release_year
                //                         .as_ref()
                //                         .unwrap_or(&"".to_string())
                //                         .to_string(),
                //                     genre: message
                //                         .genre
                //                         .as_ref()
                //                         .unwrap_or(&"".to_string())
                //                         .to_string(),
                //                 })
                //                 .unwrap();
                //             csv_writer.flush().unwrap();
                //         }
                //         CLIOutputType::SongName => {
                //             println!("{}", song_name);
                //         }
                //     };
                // }
                //if do_recognize_once {
                main_loop_cli.quit();
                return glib::Continue(false);
            }
            _ => {}
        }
        glib::Continue(true)
    });

    main_loop.run();

    Ok(())
}
