use std::{
    fs::{self, File}, io::BufReader, path::PathBuf, process::exit, sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    }, time::Duration
};

use iced::{
    time, widget::{button, column, row, scrollable, svg, text}, Element, Subscription, Task
};
use lofty::{file::TaggedFileExt, read_from_path, tag::TagItem};
use rhai::Engine;
use rodio::{Decoder, OutputStream, Sink, Source};
use seeker::SeekPos;

mod seeker;

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
    playing: bool,
    seek_value: Arc<Mutex<SeekPos>>,
    seeking: bool,
    songs: Vec<Song>,
}
fn to_string<T: ToString>(t: T) -> String {
    t.to_string()
}

fn fold_songs(mut song: Song, tag: &TagItem) -> Song {
    let tag = tag.clone();
    match tag.clone().into_key() {
        lofty::tag::ItemKey::AlbumTitle => song.album_name = tag.into_value().into_string(),
        lofty::tag::ItemKey::SetSubtitle => {}
        lofty::tag::ItemKey::ShowName => {}
        lofty::tag::ItemKey::ContentGroup => {}
        lofty::tag::ItemKey::TrackTitle => song.name = tag.into_value().into_string(),
        lofty::tag::ItemKey::TrackSubtitle => {}
        lofty::tag::ItemKey::OriginalAlbumTitle => {}
        lofty::tag::ItemKey::OriginalArtist => {}
        lofty::tag::ItemKey::OriginalLyricist => {}
        lofty::tag::ItemKey::AlbumTitleSortOrder => {}
        lofty::tag::ItemKey::AlbumArtistSortOrder => {}
        lofty::tag::ItemKey::TrackTitleSortOrder => {}
        lofty::tag::ItemKey::TrackArtistSortOrder => {}
        lofty::tag::ItemKey::ShowNameSortOrder => {}
        lofty::tag::ItemKey::ComposerSortOrder => {}
        lofty::tag::ItemKey::AlbumArtist => song.album_artist = tag.into_value().into_string(),
        lofty::tag::ItemKey::TrackArtist => song.track_artist = tag.into_value().into_string(),
        lofty::tag::ItemKey::Arranger => {}
        lofty::tag::ItemKey::Writer => {}
        lofty::tag::ItemKey::Composer => {}
        lofty::tag::ItemKey::Conductor => {}
        lofty::tag::ItemKey::Director => {}
        lofty::tag::ItemKey::Engineer => {}
        lofty::tag::ItemKey::Lyricist => {}
        lofty::tag::ItemKey::MixDj => {}
        lofty::tag::ItemKey::MixEngineer => {}
        lofty::tag::ItemKey::MusicianCredits => {}
        lofty::tag::ItemKey::Performer => {}
        lofty::tag::ItemKey::Producer => {}
        lofty::tag::ItemKey::Publisher => {}
        lofty::tag::ItemKey::Label => {}
        lofty::tag::ItemKey::InternetRadioStationName => {}
        lofty::tag::ItemKey::InternetRadioStationOwner => {}
        lofty::tag::ItemKey::Remixer => {}
        lofty::tag::ItemKey::DiscNumber => song.disc_number = tag.into_value().into_string().map(|x| x.parse().expect("failed to parse value")),
        lofty::tag::ItemKey::DiscTotal => {}
        lofty::tag::ItemKey::TrackNumber => song.track_number = tag.into_value().into_string().map(|x| x.parse().expect("failed to parse value")),
        lofty::tag::ItemKey::TrackTotal => {}
        lofty::tag::ItemKey::Popularimeter => {}
        lofty::tag::ItemKey::ParentalAdvisory => {}
        lofty::tag::ItemKey::RecordingDate => song.recording_date = tag.into_value().into_string(),
        lofty::tag::ItemKey::Year => {}
        lofty::tag::ItemKey::ReleaseDate => {}
        lofty::tag::ItemKey::OriginalReleaseDate => {}
        lofty::tag::ItemKey::Isrc => {}
        lofty::tag::ItemKey::Barcode => {}
        lofty::tag::ItemKey::CatalogNumber => {}
        lofty::tag::ItemKey::Work => {}
        lofty::tag::ItemKey::Movement => {}
        lofty::tag::ItemKey::MovementNumber => {}
        lofty::tag::ItemKey::MovementTotal => {}
        lofty::tag::ItemKey::MusicBrainzRecordingId => {}
        lofty::tag::ItemKey::MusicBrainzTrackId => {}
        lofty::tag::ItemKey::MusicBrainzReleaseId => {}
        lofty::tag::ItemKey::MusicBrainzReleaseGroupId => {}
        lofty::tag::ItemKey::MusicBrainzArtistId => {}
        lofty::tag::ItemKey::MusicBrainzReleaseArtistId => {}
        lofty::tag::ItemKey::MusicBrainzWorkId => {}
        lofty::tag::ItemKey::FlagCompilation => {}
        lofty::tag::ItemKey::FlagPodcast => {}
        lofty::tag::ItemKey::FileType => {}
        lofty::tag::ItemKey::FileOwner => {}
        lofty::tag::ItemKey::TaggingTime => {}
        lofty::tag::ItemKey::Length => {}
        lofty::tag::ItemKey::OriginalFileName => {}
        lofty::tag::ItemKey::OriginalMediaType => {}
        lofty::tag::ItemKey::EncodedBy => {}
        lofty::tag::ItemKey::EncoderSoftware => {}
        lofty::tag::ItemKey::EncoderSettings => {}
        lofty::tag::ItemKey::EncodingTime => {}
        lofty::tag::ItemKey::ReplayGainAlbumGain => {}
        lofty::tag::ItemKey::ReplayGainAlbumPeak => {}
        lofty::tag::ItemKey::ReplayGainTrackGain => {}
        lofty::tag::ItemKey::ReplayGainTrackPeak => {}
        lofty::tag::ItemKey::AudioFileUrl => {}
        lofty::tag::ItemKey::AudioSourceUrl => {}
        lofty::tag::ItemKey::CommercialInformationUrl => {}
        lofty::tag::ItemKey::CopyrightUrl => {}
        lofty::tag::ItemKey::TrackArtistUrl => {}
        lofty::tag::ItemKey::RadioStationUrl => {}
        lofty::tag::ItemKey::PaymentUrl => {}
        lofty::tag::ItemKey::PublisherUrl => {}
        lofty::tag::ItemKey::Genre => {}
        lofty::tag::ItemKey::InitialKey => {}
        lofty::tag::ItemKey::Color => {}
        lofty::tag::ItemKey::Mood => {}
        lofty::tag::ItemKey::Bpm => {}
        lofty::tag::ItemKey::IntegerBpm => {}
        lofty::tag::ItemKey::CopyrightMessage => {}
        lofty::tag::ItemKey::License => {}
        lofty::tag::ItemKey::PodcastDescription => {}
        lofty::tag::ItemKey::PodcastSeriesCategory => {}
        lofty::tag::ItemKey::PodcastUrl => {}
        lofty::tag::ItemKey::PodcastGlobalUniqueId => {}
        lofty::tag::ItemKey::PodcastKeywords => {}
        lofty::tag::ItemKey::Comment => {}
        lofty::tag::ItemKey::Description => {}
        lofty::tag::ItemKey::Language => {}
        lofty::tag::ItemKey::Script => {}
        lofty::tag::ItemKey::Lyrics => {}
        lofty::tag::ItemKey::AppleXid => {}
        lofty::tag::ItemKey::AppleId3v2ContentGroup => {}
        lofty::tag::ItemKey::Unknown(_) => {}
        _ => {}
    }
    song
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

        tokio::spawn(play_manager(rx, tx_rust));


        let seek_value = Arc::new(Mutex::new(SeekPos::from_range(0.0, 1.0)));

        let songs = fs::read_dir("assets/songs").expect("failed to read dir");
        let songs = songs.map(|song| {
            let song = song.expect("song is error");
            let path = song.path();
            let tagged_file = read_from_path(path.clone()).expect("failed to read tagged_file");
            let a = tagged_file.primary_tag();
            a
                .expect("no tag?")
                .items()
                .fold(Song::new(path.clone()), fold_songs)
        }).collect();

        (
            State {
                player_tx: tx,
                playing: false,
                seek_value,
                seeking: false,
                songs,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Play => {
                self.player_tx
                    .send(PlayerMessage::Play)
                    .expect("failed to send play message");
                self.playing = true;
                Task::none()
            }
            Message::Pause => {
                self.player_tx
                    .send(PlayerMessage::Paus)
                    .expect("failed to send paus message");
                self.playing = false;
                Task::none()
            }
            Message::Next => {
                self.player_tx
                    .send(PlayerMessage::Next)
                    .expect("failed to send next message");
                Task::none()
            }
            Message::Prev => {
                self.player_tx
                    .send(PlayerMessage::Prev)
                    .expect("failed to send prev message");
                Task::none()
            }
            Message::SeekChanged(val) => {
                println!("seeking: {:?}, currently at: {:?}", val, self.seek_value);
                self.player_tx
                    .send(PlayerMessage::Seek(val))
                    .expect("failed to send seek message");
                let mut seek_value = self.seek_value.lock().unwrap();
                *seek_value = val;
                Task::none()
            }
            Message::SeekUpdate => {
                let sv = self.seek_value.clone();
                match self.player_tx
                    .send(PlayerMessage::GetPos(Box::new(move |pos| {
                        // println!("get pos from player {:?}", pos);
                        let mut seek_value = sv.lock().unwrap();
                        *seek_value = pos;
                    }))) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("error sending seekupdate {}", e);
                        }
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
                println!("selected song {:?}", song.name);
                self.player_tx.send(PlayerMessage::PlaySong(song)).expect("failed to send playsong message");
                Task::done(Message::Play)
            },
        }
    }
    fn view(&self) -> Element<Message> {
        column![
            play_controls(self.playing),
            // seek_bar(*self.seek_value.lock().expect("mutex failed to lock")),
            seeker::seeker(
                *self.seek_value.lock().expect("mutex failed to lock"),
                self.player_tx.clone(),
            ),
            song_browser(&self.songs)
        ]
        .into()
    }
    fn subscription(&self) -> Subscription<Message> {
        if ! self.seeking {
            
            time::every(Duration::from_millis(100)).map(|_| Message::SeekUpdate)
        } else {
            Subscription::none()
        }
    }
}

fn song_browser(songs: &Vec<Song>) -> Element<'static, Message> {
    let name_width = 200.0;
    let artist_width = 150.0;
    // clones are not good
    scrollable(
        column(songs.into_iter().map(move |s| {
            song(s.clone(), name_width, artist_width)
        }))
    ).into()
}

fn song(song: Song, name_width: f32, artist_width: f32) -> Element<'static, Message> {
    button(row![
            text(song.name.as_ref().expect("no song name").clone()).width(name_width),
            text(song.track_artist.as_ref().expect("no song name").clone()).width(artist_width),
        ])
        .on_press_with(move ||{
            Message::SongSelected(song.clone())
        }).into()
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

async fn play_manager(rx: Receiver<PlayerMessage>, tx_rust: Sender<String>) {
    let (_stream, stream_handle) =
        OutputStream::try_default().expect("could not create default OutputStream");
    let sink = Sink::try_new(&stream_handle).expect("could not create new Sink");

    let file = BufReader::new(File::open("assets/songs/test.flac").expect("failed to load test file"));
    let source = Decoder::new(file).expect("failed to create decoder from test file");
    let mut duration = source
        .total_duration()
        .expect("failed to get souce duration");
    sink.append(source);
    sink.pause();

    for msg in rx {
        match msg {
            PlayerMessage::Play => {
                println!("playing");
                sink.play();
                tx_rust
                    .send("play".to_string())
                    .expect("failed to send message to rhai");
            }
            PlayerMessage::Stop => {
                println!("stoped");
                sink.pause();
                tx_rust
                    .send("stop".to_string())
                    .expect("failed to send message to rhai");
            }
            PlayerMessage::Seek(place) => {
                println!("seeking {:?}", place);
                let pos = place.get() * duration.as_secs_f64();
                sink.try_seek(Duration::from_secs_f64(pos))
                    .expect("could not seek");
                tx_rust
                    .send("seek".to_string())
                    .expect("failed to send message to rhai");
            }
            PlayerMessage::GetPos(mut call_back) => {
                let pos = sink.get_pos();
                let seek_pos = SeekPos::from_secs_percent(pos.as_secs_f64(), duration);
                call_back(seek_pos);
            }
            PlayerMessage::Paus => {
                println!("paused");
                sink.pause();
                tx_rust
                    .send("paus".to_string())
                    .expect("failed to send message to rhai");
            }
            PlayerMessage::Next => {
                println!("next");
                println!("not yet working");
                tx_rust
                    .send("next".to_string())
                    .expect("failed to send message to rhai");
            }
            PlayerMessage::Prev => {
                println!("prev");
                println!("not yet working");
                tx_rust
                    .send("prev".to_string())
                    .expect("failed to send message to rhai");
            }
            PlayerMessage::PlaySong(song) => {
                let file = BufReader::new(File::open(song.path).expect("failed to load test file"));
                let source = Decoder::new(file).expect("failed to create decoder from test file");
                duration = source
                    .total_duration()
                    .expect("failed to get souce duration");
                sink.append(source);
                // sink.skip_one();
            },
        }
    }
}
