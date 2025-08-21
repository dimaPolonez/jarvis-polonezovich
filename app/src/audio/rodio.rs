/*
    Abandoned temporary.
    Problems with blocking behaviour.
    Possible fixes are running rodio in a separate thread or smthng.
*/

use std::fs::File;
use std::path::PathBuf;
use std::io::BufReader;
use once_cell::sync::OnceCell;
use std::cell::RefCell;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};

static STREAM_HANDLE: OnceCell<OutputStreamHandle> = OnceCell::new();
static SINK: OnceCell<Sink> = OnceCell::new();

thread_local!(static STREAM_TL: RefCell<Option<OutputStream>> = RefCell::new(None));

pub fn init() -> Result<(), ()> {
    if !STREAM_HANDLE.get().is_none() {return Ok(());} // already initialized

    // get output stream handle to the default physical sound device
    match OutputStream::try_default() {
        Ok(out) => {
            // divide
            let (_stream, stream_handle) = out;

            // create sink
            let sink;
            match Sink::try_new(&stream_handle) {
                Ok(s) => {
                    info!("Sink initialized.");
                    sink = s;
                },
                Err(msg) => {
                    error!("Cannot create sink.\nError details: {}", msg);

                    // failed
                    return Err(())
                }
            }

            // store
            STREAM_TL.with(|s| {
                *s.borrow_mut() = Some(_stream);
            });
            let _ = STREAM_HANDLE.set(stream_handle);
            let _ = SINK.set(sink);

            // success
            Ok(())
        },
        Err(msg) => {
            error!("Failed to initialize audio stream.\nError details: {}", msg);

            // failed
            Err(())
        }
    }
}

pub fn play_sound(filename: &PathBuf, sleep: bool) {
    // ensure initialized
    if STREAM_HANDLE.get().is_none() || SINK.get().is_none() {
        match init() {
            Ok(_) => (),
            Err(_) => {
                error!("Rodio init failed; cannot play {}", filename.display());
                return;
            }
        }
    }

    // Load a sound from a file
    let file = match File::open(&filename) {
        Ok(f) => BufReader::new(f),
        Err(e) => {
            warn!("Cannot open sound file {}: {}", filename.display(), e);
            return;
        }
    };

    // Decode that sound file into a source
    let source = match Decoder::new(file) {
        Ok(src) => src,
        Err(e) => {
            warn!("Cannot decode sound file {}: {}", filename.display(), e);
            return;
        }
    };

    // Play the sound directly on the device
    if let Some(sink) = SINK.get() {
        sink.append(source);
        if sleep {
            sink.sleep_until_end();
        }
    } else {
        error!("Rodio sink is not initialized; cannot play {}", filename.display());
    }
}