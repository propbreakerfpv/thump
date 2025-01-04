use std::{
    fmt::Debug,
    time::Duration,
};

use rodio::{OutputStream, Sink};


pub struct PlayerManager {
    pub sink: Sink,
    _stream: OutputStream,
    pub duration: Duration,
}

impl PlayerManager {
    pub fn new() -> PlayerManager {
        let (_stream, stream_handle) =
            OutputStream::try_default().expect("could not create default OutputStream");
        let sink = Sink::try_new(&stream_handle).expect("could not create new Sink");
        PlayerManager {
            sink,
            _stream,
            duration: Duration::from_secs(1),
        }
    }
}

impl Debug for PlayerManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

