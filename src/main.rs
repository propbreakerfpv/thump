use std::{
    collections::VecDeque,
    fs::File,
    io::BufReader,
    path::PathBuf,
    process::exit,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    time::Duration,
};

use iced::{
    border::top, futures::Stream, time, widget::{button, column, row, scrollable, svg, text}, Element, Subscription, Task
};
use play_manager::PlayerManager;
use read_files::search_dir;
use rhai::Engine;
use rodio::{Decoder, OutputStream, Sink, Source};
use seeker::SeekPos;

mod read_files;
mod seeker;
mod play_manager;

const NEXT_ICON: &[u8; 1714] = include_bytes!("../assets/next.svg");
const PREV_ICON: &[u8; 1707] = include_bytes!("../assets/prev.svg");
const PLAY_ICON: &[u8; 859] = include_bytes!("../assets/play.svg");
const PAUSE_ICON: &[u8; 1624] = include_bytes!("../assets/pause.svg");

#[derive(Debug, Clone)]
pub struct Song {
    name: Option<String>,
    album_artist: Option<String>,
    track_artist: Option<String>,
    recording_date: Option<String>,
    track_number: Option<i32>,
    disc_number: Option<i32>,
    album_name: Option<String>,
    path: PathBuf,
}

impl Song {
    fn new(path: PathBuf) -> Song {
        Song {
            path,
            name: None,
            album_artist: None,
            track_artist: None,
            recording_date: None,
            track_number: None,
            disc_number: None,
            album_name: None,
        }
    }
}

#[tokio::main]
async fn main() {
    iced::application("Thump", State::update, State::view)
        .subscription(State::subscription)
        .run_with(State::new)
        .unwrap();
    exit(1)
}

enum PlayerMessage {
    Play,
    Paus,
    Stop,
    Next,
    Prev,
    Seek(SeekPos),
    GetPos(Box<dyn FnMut(SeekPos) + Send>),
    PlaySong(Song),
}
unsafe impl Send for PlayerMessage { }

#[derive(Debug, Clone)]
enum Message {
    Play,
    Pause,
    Next,
    Prev,
    SeekUpdate,
    SeekChanged(SeekPos),
    Seeking,
    DoneSeeking,
    SongSelected(Song),
}

#[derive(Debug)]
struct State {
    player_tx: Sender<PlayerMessage>,
    tx_rust: Sender<String>,
    player_manager: PlayerManager,
    playing: bool,
    seek_value: SeekPos,
    seeking: bool,
    songs: Vec<Song>,
    now_playing: Option<Song>,
    player_que: VecDeque<Song>,
}

impl State {
    fn new() -> (State, Task<Message>) {
        let (tx, rx) = channel::<PlayerMessage>();
        let (tx_rust, rx_rhai) = channel();
        let (tx_rhai, rx_rust) = channel();
        tokio::spawn(async move {
            let mut engine = Engine::new();
            engine
                .register_fn("get", move || rx_rhai.recv().unwrap_or_default())
                .register_fn("put", move |v: String| tx_rhai.send(v).unwrap());

            engine
                .run(
                    r#"
        print("from script");
        loop {
            let value = get();
            // print(`got ${value}`);
        }
        "#,
                )
                .expect("failed to run script");
        });

        // tokio::spawn(play_manager(rx, tx_rust));

        let seek_value = SeekPos::from_range(0.0, 1.0);

        let songs = search_dir("/Users/jonas/Soulseek Downloads/complete");
        songs.iter().for_each(|s| {
            println!("song: {:?} {:?}", s.name, s.track_artist);
        });



        (
            State {
                player_tx: tx,
                tx_rust,
                player_manager: PlayerManager::new(),
                playing: false,
                seek_value,
                seeking: false,
                songs,
                now_playing: None,
                player_que: VecDeque::new(),
            },
            Task::none()
            // Task::future(play_manager(rx, tx_rust)),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Play => {
                println!("playing msg received. empty: {}", self.player_manager.sink.empty());
                self.player_manager.sink.play();
                self.tx_rust
                    .send("play".to_string())
                    .expect("failed to send message to rhai");
                self.playing = true;
                Task::none()
            }
            Message::Pause => {
                println!("paused");
                self.player_manager.sink.pause();
                self.tx_rust
                    .send("paus".to_string())
                    .expect("failed to send message to rhai");
                self.playing = false;
                Task::none()
            }
            Message::Next => {
                println!("next");
                // println!("not yet working");
                self.player_manager.sink.skip_one();
                self.tx_rust
                    .send("next".to_string())
                    .expect("failed to send message to rhai");
                Task::none()
            }
            Message::Prev => {
                println!("prev");
                println!("not yet working");
                self.tx_rust
                    .send("prev".to_string())
                    .expect("failed to send message to rhai");
                Task::none()
            }
            Message::SeekChanged(val) => {
                println!("seeking {:?}", val);
                let pos = val.get() * self.player_manager.duration.as_secs_f64();
                self.player_manager.sink.try_seek(Duration::from_secs_f64(pos))
                    .expect("could not seek");
                self.tx_rust
                    .send("seek".to_string())
                    .expect("failed to send message to rhai");
                self.seek_value = val;
                Task::none()
            }
            Message::SeekUpdate => {

                let pos = self.player_manager.sink.get_pos();
                // println!("getpos {:?}", pos);
                self.seek_value = SeekPos::from_secs_percent(pos.as_secs_f64(), self.player_manager.duration);
                if self.seek_value.get() >= 0.999999 {
                    println!("next_song");
                }
                Task::none()
            }
            Message::Seeking => {
                self.seeking = true;
                Task::none()
            }
            Message::DoneSeeking => {
                self.seeking = false;
                Task::none()
            }
            Message::SongSelected(song) => {
                let file =
                BufReader::new(File::open(song.path.clone()).expect("failed to load test file"));
                let source =
                Decoder::new(file).expect("failed to create decoder from test file");
                // self.duration = source
                //     .total_duration()
                //     .expect("failed to get souce duration");
                let duration = source.total_duration().expect("failed to get source duration");
                self.player_manager.sink.append(source);
                // Message::SetDuration(duration)
                // return Subscription::none().map(move |_: ()| Message::SetDuration(duration));
                self.player_manager.duration = duration;
                self.player_que.push_back(song);
                Task::done(Message::Play)
            }
        }
    }
    fn view(&self) -> Element<Message> {
        column![
            play_controls(self.playing),
            // seek_bar(*self.seek_value.lock().expect("mutex failed to lock")),
            seeker::seeker(
                self.seek_value,
                self.player_tx.clone(),
            ),
            now_playing(),
            song_browser(&self.songs)
        ]
        .into()
    }
    fn subscription(&self) -> Subscription<Message> {
        let seeking = if !self.seeking {
            time::every(Duration::from_millis(100)).map(|_| Message::SeekUpdate)
        } else {
            Subscription::none()
        };
        Subscription::batch([seeking /* self.player_manager.player_subscription() */])
    }
}

fn song_browser(songs: &Vec<Song>) -> Element<'static, Message> {
    let name_width = 200.0;
    let artist_width = 150.0;
    // clones are not good
    scrollable(column(
        songs
            .into_iter()
            .map(move |s| song(s.clone(), name_width, artist_width)),
    ))
    .into()
}

fn song(song: Song, name_width: f32, artist_width: f32) -> Element<'static, Message> {
    button(row![
        text(song.name.as_ref().expect("no song name").clone()).width(name_width),
        text(song.track_artist.as_ref().expect("no song name").clone()).width(artist_width),
    ])
    .on_press_with(move || Message::SongSelected(song.clone()))
    .into()
}

fn now_playing() -> Element<'static, Message> {
    text("now playing").into()
}

fn play_controls(playing: bool) -> Element<'static, Message> {
    let next_handle = svg::Handle::from_memory(NEXT_ICON);
    let prev_handle = svg::Handle::from_memory(PREV_ICON);
    let play_handle = svg::Handle::from_memory(PLAY_ICON);
    let pause_handle = svg::Handle::from_memory(PAUSE_ICON);

    let play_btn = if playing {
        button(svg(pause_handle).width(25).height(25)).on_press(Message::Pause)
    } else {
        button(svg(play_handle).width(25).height(25)).on_press(Message::Play)
    };

    row![
        button(svg(prev_handle).width(25).height(25)).on_press(Message::Prev),
        play_btn,
        button(svg(next_handle).width(25).height(25)).on_press(Message::Next),
    ]
    .into()
}
