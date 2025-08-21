mod rodio;
mod kira;

use std::cmp::Ordering;
use std::path::PathBuf;
use std::ffi::OsStr;
use once_cell::sync::OnceCell;

use crate::{config, DB, SOUND_DIR};
use crate::listener;
use crate::config::structs::AudioType;

static AUDIO_TYPE: OnceCell<AudioType> = OnceCell::new();


pub fn init() -> Result<(), ()> {
    if !AUDIO_TYPE.get().is_none() {return Ok(());} // already initialized

    // set default audio type
    // @TODO. Make it configurable?
    AUDIO_TYPE.set(config::DEFAULT_AUDIO_TYPE).unwrap();

    // load given audio backend
    match AUDIO_TYPE.get().unwrap() {
        AudioType::Rodio => {
            // Init Rodio
            info!("Initializing Rodio audio backend.");

            match rodio::init() {
                Ok(_) => {
                    info!("Successfully initialized Rodio audio backend.");
                },
                Err(msg) => {
                    error!("Failed to initialize Rodio audio backend.");

                    return Err(())
                }
            }
        },
        AudioType::Kira => {
            // Init Kira
            info!("Initializing Kira audio backend.");

            match kira::init() {
                Ok(_) => {
                    info!("Successfully initialized Kira audio backend.");
                },
                Err(msg) => {
                    error!("Failed to initialize Kira audio backend.");

                    return Err(())
                }
            }
        }
    }

    Ok(())
}

pub fn resolve_sound_path(name: &str) -> Option<PathBuf> {
    // try current voice dir first
    if let Some(dir) = get_sound_directory() {
        let p = dir.join(name);
        if p.exists() { return Some(p); }
    }
    // then try root sound dir
    let root = SOUND_DIR.join(name);
    if root.exists() { return Some(root); }
    None
}

pub fn play_sound(filename: &PathBuf) {
    info!("Playing {}", filename.display());

    // mute wake-word while playing sound to avoid self-trigger
    listener::mute_wake_for(config::PLAYBACK_WAKE_MUTE);

    let is_mp3 = filename.extension().and_then(OsStr::to_str).map(|ext| ext.eq_ignore_ascii_case("mp3")).unwrap_or(false);

    if is_mp3 {
        // prefer rodio for mp3
        rodio::play_sound(filename, false);
        return;
    }

    match AUDIO_TYPE.get().unwrap() {
        AudioType::Rodio => {
            rodio::play_sound(filename, true);
        },
        AudioType::Kira => {
            kira::play_sound(filename)
        }
    }
}

pub fn play_sound_blocking(filename: &PathBuf) {
    info!("Playing (blocking) {}", filename.display());

    // mute wake-word while playing sound to avoid self-trigger
    listener::mute_wake_for(config::PLAYBACK_WAKE_MUTE);

    let is_mp3 = filename.extension().and_then(OsStr::to_str).map(|ext| ext.eq_ignore_ascii_case("mp3")).unwrap_or(false);

    if is_mp3 {
        // for notifications, block until end to ensure audibility
        rodio::play_sound(filename, true);
        return;
    }

    match AUDIO_TYPE.get().unwrap() {
        AudioType::Rodio => {
            rodio::play_sound(filename, true);
        },
        AudioType::Kira => {
            kira::play_sound(filename)
        }
    }
}

pub fn get_sound_directory() -> Option<PathBuf> {
    let voice = DB.get().unwrap().voice.as_str();
    let voice_path = SOUND_DIR.join(voice);

    match voice_path.exists() && voice_path.cmp(&SOUND_DIR) != Ordering::Equal {
        true => Some(voice_path),
        _ => {
            let default_voice_path = SOUND_DIR.join(config::DEFAULT_VOICE);

            match default_voice_path.exists() {
                true => Some(default_voice_path),
                _ => None
            }
        }
    }
}