use std::{
    fs::File,
    io::BufReader,
    process::exit,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    time::Duration,
};

use iced::{
    time,
    widget::{button, column, row, slider, svg},
    Element, Subscription, Task,
};
use rhai::Engine;
use rodio::{Decoder, OutputStream, Sink, Source};
use seeker::{SeekPos, Seeker};

mod seeker;

const SEEK_DEVIDER: i32 = 1000000;

const NEXT_ICON: &[u8; 1714] = include_bytes!("../assets/next.svg");
const PREV_ICON: &[u8; 1707] = include_bytes!("../assets/prev.svg");
const PLAY_ICON: &[u8; 859] = include_bytes!("../assets/play.svg");
const PAUSE_ICON: &[u8; 1624] = include_bytes!("../assets/pause.svg");

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
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Play,
    Pause,
    Next,
    Prev,
    SeekUpdate,
    SeekChanged(SeekPos),
    Seeking,
    DoneSeeking,
}

#[derive(Debug)]
struct State {
    player_tx: Sender<PlayerMessage>,
    playing: bool,
    seek_value: Arc<Mutex<SeekPos>>,
    seeking: bool
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
            print(`got ${value}`);
        }
        "#,
                )
                .expect("failed to run script");
        });

        tokio::spawn(play_manager(rx, tx_rust));

        // manage seeker
        // let seek_value = Mutex::new(0.0);
        let seek_value = Arc::new(Mutex::new(SeekPos::from_range(0.0, 1.0)));
        let seeker_value = seek_value.clone();
        let seeker_tx = tx.clone();
        // tokio::spawn(async move {
        //     for _ in 0.. {
        //         let sv = seeker_value.clone();
        //         seeker_tx
        //             .send(PlayerMessage::GetPos(Box::new(move |pos| {
        //                 let mut sv = sv.lock().expect("failed to get mut arc");
        //                 *sv = ((pos * SEEK_DEVIDER as f64) as u64) as f64;
        //                 println!("just changed seek to: {}", sv);
        //             })))
        //             .expect("failed to send getpos message");
        //         sleep(Duration::from_millis(50));
        //     }
        // });

        (
            State {
                player_tx: tx,
                playing: false,
                seek_value,
                seeking: false,
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
                self.player_tx
                    .send(PlayerMessage::GetPos(Box::new(move |pos| {
                        // println!("get pos from player {:?}", pos);
                        let mut seek_value = sv.lock().unwrap();
                        *seek_value = pos;
                    })))
                    .expect("failed to send getPos message");
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
        }
    }
    fn view(&self) -> Element<Message> {
        column![
            play_controls(self.playing),
            // seek_bar(*self.seek_value.lock().expect("mutex failed to lock")),
            seeker::seeker(
                *self.seek_value.lock().expect("mutex failed to lock"),
                self.player_tx.clone(),
            )
        ]
        .into()
    }
    fn subscription(&self) -> Subscription<Message> {
        if ! self.seeking {
            let tick = time::every(Duration::from_millis(100)).map(|_| Message::SeekUpdate);
            tick
        } else {
            Subscription::none()
        }
    }
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

    let file = BufReader::new(File::open("assets/test.flac").expect("failed to load test file"));
    let source = Decoder::new(file).expect("failed to create decoder from test file");
    let duration = source
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
        }
    }
}
