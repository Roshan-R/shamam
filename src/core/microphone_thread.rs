use async_channel::Sender;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

#[cfg(target_os = "linux")]
use gag::Gag;

use crate::core::thread_messages::{MicrophoneMessage::*, *};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use gettextrs::gettext;

pub fn microphone_thread(
    microphone_rx: mpsc::Receiver<MicrophoneMessage>,
    processing_tx: mpsc::Sender<ProcessingMessage>,
    gui_tx: Sender<GUIMessage>,
) {
    // Use the default host for working with audio devices.

    let host = cpal::default_host();

    // Run the input stream on a separate thread.

    let mut stream: Option<cpal::Stream> = None;

    let processing_already_ongoing: Arc<Mutex<bool>> = Arc::new(Mutex::new(false)); // Whether our data is already being processed in other threads (pointer to a bool shared between this thread and the CPAL thread, hence the Arc<Mutex>)

    // Send a list of the active microphone-alike devices to the GUI thread
    // (the combo box will be filed with device names when a "DevicesList"
    // inter-thread message will be received at the initialization of the
    // microphone thread, because CPAL which underlies Rodio can't be called
    // from the same thread as the microphone thread under Windows, see:
    //  - https://github.com/RustAudio/rodio/issues/270
    //  - https://github.com/RustAudio/rodio/issues/214 )

    // Avoid having alsalib polluting stderr (https://github.com/RustAudio/cpal/issues/384)
    // through disabling stderr temporarily

    #[cfg(target_os = "linux")]
    let print_gag = Gag::stderr().unwrap();

    let mut device_names: Vec<String> = vec![];

    for device in host.input_devices().unwrap() {
        let device_name = device.name().unwrap();

        // Selecting the "upmix" or "vdownmix" input
        // source on an ALSA-based configuration may
        // crash our underlying sound library.

        if device_name.contains("upmix") || device_name.contains("downmix") {
            continue;
        }

        device_names.push(device_name);
    }

    dbg!(&device_names);

    gui_tx
        .send_blocking(GUIMessage::DevicesList(Box::new(device_names)))
        .unwrap();

    #[cfg(target_os = "linux")]
    drop(print_gag);

    // Process ingress inter-thread messages (stopping or starting
    // recording from the microphone, and knowing from which device
    // in particular)

    for message in microphone_rx.iter() {
        match message {
            MicrophoneRecordStart(device_name) => {
                let processing_tx_2 = processing_tx.clone();
                let gui_tx_2 = gui_tx.clone();
                let gui_tx_3 = gui_tx.clone();
                let gui_tx_4 = gui_tx.clone();

                let err_fn = move |error| {
                    gui_tx_2
                        .send_blocking(GUIMessage::ErrorMessage(format!(
                            "{} {}",
                            gettext("Microphone error:"),
                            error
                        )))
                        .unwrap();
                };

                let mut device: cpal::Device = host.default_input_device().unwrap();

                // Avoid having alsalib polluting stderr (https://github.com/RustAudio/cpal/issues/384)
                // through disabling stderr temporarily

                #[cfg(target_os = "linux")]
                let print_gag = Gag::stderr().unwrap();

                for possible_device in host.input_devices().unwrap() {
                    if possible_device.name().unwrap() == device_name {
                        device = possible_device;
                        break;
                    }
                }

                #[cfg(target_os = "linux")]
                drop(print_gag);

                let config = device
                    .default_input_config()
                    .expect(&gettext("Failed to get default input config"));

                let channels = config.channels();
                let sample_rate = config.sample_rate().0;

                let mut twelve_seconds_buffer: [i16; 16000 * 12] = [0; 16000 * 12];
                let mut number_unprocessed_samples: usize = 0; // Sample count for the interval of doing Shazam recognition (every 4 seconds)
                let mut number_unmeasured_samples: usize = 0; // Sample count for doing volume measurement (every 24th of second)

                let processing_already_ongoing_2 = processing_already_ongoing.clone();

                stream = Some(match config.sample_format() {
                    cpal::SampleFormat::F32 => device
                        .build_input_stream(
                            &config.into(),
                            move |data, _: &_| {
                                write_data::<f32, f32>(
                                    data,
                                    &processing_tx_2,
                                    gui_tx_3.clone(),
                                    channels,
                                    sample_rate,
                                    &mut twelve_seconds_buffer,
                                    &mut number_unprocessed_samples,
                                    &mut number_unmeasured_samples,
                                    &processing_already_ongoing_2,
                                )
                            },
                            err_fn,
                        )
                        .unwrap(),
                    cpal::SampleFormat::I16 => device
                        .build_input_stream(
                            &config.into(),
                            move |data, _: &_| {
                                write_data::<i16, i16>(
                                    data,
                                    &processing_tx_2,
                                    gui_tx_3.clone(),
                                    channels,
                                    sample_rate,
                                    &mut twelve_seconds_buffer,
                                    &mut number_unprocessed_samples,
                                    &mut number_unmeasured_samples,
                                    &processing_already_ongoing_2,
                                )
                            },
                            err_fn,
                        )
                        .unwrap(),
                    cpal::SampleFormat::U16 => device
                        .build_input_stream(
                            &config.into(),
                            move |data, _: &_| {
                                write_data::<u16, i16>(
                                    data,
                                    &processing_tx_2,
                                    gui_tx_3.clone(),
                                    channels,
                                    sample_rate,
                                    &mut twelve_seconds_buffer,
                                    &mut number_unprocessed_samples,
                                    &mut number_unmeasured_samples,
                                    &processing_already_ongoing_2,
                                )
                            },
                            err_fn,
                        )
                        .unwrap(),
                });

                stream.as_ref().unwrap().play().unwrap();

                gui_tx_4
                    .send_blocking(GUIMessage::MicrophoneRecording)
                    .unwrap();
            }

            MicrophoneRecordStop => {
                drop(stream.unwrap());

                stream = None;
            }

            ProcessingDone => {
                let mut processing_already_ongoing_borrow =
                    processing_already_ongoing.lock().unwrap();
                *processing_already_ongoing_borrow = false;
            }
        }
    }
}

fn write_data<T, U>(
    input_samples: &[T],
    processing_tx: &mpsc::Sender<ProcessingMessage>,
    gui_tx: Sender<GUIMessage>,
    channels: u16,
    sample_rate: u32,
    twelve_seconds_buffer: &mut [i16],
    number_unprocessed_samples: &mut usize,
    number_unmeasured_samples: &mut usize,
    processing_already_ongoing: &Arc<Mutex<bool>>,
) where
    T: cpal::Sample + rodio::Sample,
    U: cpal::Sample,
{
    // Reassemble data into a 12-samples buffer, and do recognition
    // every 4 seconds if the queue to "processing_tx" is empty

    let input_buffer =
        rodio::buffer::SamplesBuffer::new::<&[T]>(channels, sample_rate, input_samples);

    let converted_file = rodio::source::UniformSourceIterator::new(input_buffer, 1, 16000);

    let raw_pcm_samples: Vec<i16> = converted_file.collect();

    if raw_pcm_samples.len() >= 16000 * 12 {
        twelve_seconds_buffer[..16000 * 12]
            .copy_from_slice(&raw_pcm_samples[raw_pcm_samples.len() - 16000 * 12..]);
    } else {
        let latter_data = twelve_seconds_buffer[raw_pcm_samples.len()..].to_vec();

        twelve_seconds_buffer[..16000 * 12 - raw_pcm_samples.len()].copy_from_slice(&latter_data);
        twelve_seconds_buffer[16000 * 12 - raw_pcm_samples.len()..]
            .copy_from_slice(&raw_pcm_samples);
    }

    *number_unprocessed_samples += raw_pcm_samples.len();

    let mut processing_already_ongoing_borrow = processing_already_ongoing.lock().unwrap();

    if *number_unprocessed_samples >= 16000 * 4 && *processing_already_ongoing_borrow == false {
        processing_tx
            .send(ProcessingMessage::ProcessAudioSamples(Box::new(
                twelve_seconds_buffer.to_vec(),
            )))
            .unwrap();

        *number_unprocessed_samples = 0;
        *processing_already_ongoing_borrow = true;
    }

    // Do microphone volume measurement every 24th of second (so that we can
    // update it at 24 FPS) and over the last two 100th of second (so that we
    // can be sure to measure volume for at most 100 Hz)

    *number_unmeasured_samples += raw_pcm_samples.len();

    if *number_unmeasured_samples >= 16000 / 24 {
        let mut max_s16le_amplitude = 1;

        for index in 16000 * 12 - 16000 / 100 * 2..16000 * 12 {
            if twelve_seconds_buffer[index] > max_s16le_amplitude {
                max_s16le_amplitude = twelve_seconds_buffer[index];
            }
        }

        let max_s16le_volume_fraction = max_s16le_amplitude as f32 / 32767.0; // 32767 is the maximum value for an i16 (2**15 - 1)

        gui_tx
            .send_blocking(GUIMessage::MicrophoneVolumePercent(
                max_s16le_volume_fraction * 100.0,
            ))
            .unwrap();

        *number_unmeasured_samples = 0;
    }
}
