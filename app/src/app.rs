use std::time::SystemTime;

use crate::{config, audio, recorder, listener, stt, commands, COMMANDS_LIST};
use rand::seq::SliceRandom;

pub fn start() -> Result<(), ()> {
    // start the loop
    main_loop()
}

fn main_loop() -> Result<(), ()> {
    let mut start: SystemTime;
    let sounds_directory = audio::get_sound_directory().unwrap();
    let frame_length: usize = recorder::frame_length(); // use recorder's frame length
    let mut frame_buffer: Vec<i16> = vec![0; frame_length];

    // play some run phrase
    // @TODO. Different sounds? Or better make it via commands or upcoming events system.
    audio::play_sound(&sounds_directory.join("run.wav"));

    // start recording
    match recorder::start_recording() {
        Ok(_) => info!("Recording started."),
        Err(_) => {
            error!("Cannot start recording.");
            return Err(()); // quit
        }
    }

    // the loop
    'wake_word: loop {
        // read from microphone
        recorder::read_microphone(&mut frame_buffer);

        // recognize wake-word
        match listener::data_callback(&frame_buffer) {
            Some(keyword_index) => {
                info!("Wake word detected (index: {})", keyword_index);
                // wake-word activated, process further commands
                // capture current time
                start = SystemTime::now();

                // activation sound
                if let Some(p) = audio::resolve_sound_path("activate.mp3") {
                    info!("Activation sound: {}", p.display());
                    audio::play_sound(&p);
                } else {
                    let p = sounds_directory.join("ok1.wav");
                    info!("Activation sound (fallback): {}", p.display());
                    audio::play_sound(&p);
                }

                // wait for voice commands
                'voice_recognition: loop {
                    info!("Entering listening mode");
                    // read from microphone
                    recorder::read_microphone(&mut frame_buffer);

                    // stt part (without partials)
                    // guard: skip STT for a short warmup after activation to avoid self-trigger
                    if start.elapsed().unwrap_or_default() < config::CMS_WARMUP_MUTE { continue 'voice_recognition; }
                    if let Some(mut recognized_voice) = stt::recognize(&frame_buffer, false) {
                        // something was recognized
                        info!("Recognized voice: {}", recognized_voice);

                        // filter recognized voice
                        // @TODO. Better recognized voice filtration.
                        recognized_voice = recognized_voice.to_lowercase();
                        for tbr in config::ASSISTANT_PHRASES_TBR {
                            recognized_voice = recognized_voice.replace(tbr, "");
                        }
                        recognized_voice = recognized_voice.trim().into();

                        // ignore empty after filtration; keep listening until timeout
                        if recognized_voice.is_empty() {
                            continue 'voice_recognition;
                        }

                        // infer command
                        if let Some((cmd_path, cmd_config)) = commands::fetch_command(&recognized_voice, &COMMANDS_LIST.get().unwrap()) {
                            // some debug info
                            info!("Recognized voice (filtered): {}", recognized_voice);
                            info!("Command found: {:?}", cmd_path);
                            info!("Executing!");

                            // execute the command
                            match commands::execute_command(&cmd_path, &cmd_config) {
                                Ok(_chain) => {
                                    // success: always stop listening and return to wake mode
                                    info!("Command executed successfully. Returning to wake mode");
                                    break 'voice_recognition;
                                },
                                Err(msg) => {
                                    // fail -> stop listening silently
                                    error!("Error executing command: {}", msg);
                                    break 'voice_recognition;
                                }
                            }
                        }
                        // no command matched -> keep listening until timeout
                        debug!("No command matched; continue listening");
                        continue 'voice_recognition;
                    }

                    // only recognize voice for a certain period of time
                    match start.elapsed() {
                        Ok(elapsed) if elapsed > config::CMS_WAIT_DELAY => {
                            // return to wake-word listening after N seconds
                            let mut played = false;
                            if let Some(p) = audio::resolve_sound_path("cancel.mp3") { // current voice
                                info!("Cancel sound on timeout: {}", p.display());
                                audio::play_sound_blocking(&p);
                                played = true;
                            }
                            if !played {
                                let p = crate::SOUND_DIR.join(config::DEFAULT_VOICE).join("cancel.mp3");
                                if p.exists() {
                                    info!("Cancel sound on timeout (default voice): {}", p.display());
                                    audio::play_sound_blocking(&p);
                                    played = true;
                                }
                            }
                            if !played {
                                let p = sounds_directory.join("not_found.wav"); // current voice wav
                                if p.exists() {
                                    info!("Cancel sound on timeout (fallback wav): {}", p.display());
                                    audio::play_sound_blocking(&p);
                                    played = true;
                                }
                            }
                            if !played {
                                let p = crate::SOUND_DIR.join(config::DEFAULT_VOICE).join("not_found.wav");
                                if p.exists() {
                                    info!("Cancel sound on timeout (default wav): {}", p.display());
                                    audio::play_sound_blocking(&p);
                                } else {
                                    warn!("No cancel sound found in any location.");
                                }
                            }
                            info!("Listening timeout reached, returning to wake mode");
                            break 'voice_recognition;
                        },
                        _ => ()
                    }
                }
            },
            None => ()
        }
    }

    Ok(())
}

fn keyword_callback(keyword_index: i32) {

}

pub fn close(code: i32) {
    info!("Closing application.");
    std::process::exit(code);
}