use std::{
    fs::File,
    io::{self, BufReader},
    sync::mpsc::{channel, Receiver},
    time::Duration,
};

use fltk::{
    app::sleep,
    button::Button,
    draw,
    enums::{Color, FrameType},
    frame::Frame,
    prelude::*,
    valuator::{Slider, SliderType},
    window::Window,
};
use fltk_theme::colors::html;
use rodio::{Decoder, OutputStream, Sink, Source};

enum PlayerMessage {
    Play,
    Stop,
    Seek(f64),
    Quit,
    GetPos(Box<dyn FnMut(f64) + Send>),
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let (tx, rx) = channel::<PlayerMessage>();
    tokio::spawn(play_manager(rx));

    let app = fltk::app::App::default();
    // let theme = ColorTheme::new(color_themes::BLACK_THEME);
    // theme.apply();
    // let scheme = WidgetScheme::new(fltk_theme::SchemeType::Clean);
    // scheme.apply();

    let mut win = Window::new(100, 100, 400, 300, "hello");
    let mut frame = Frame::new(0, 0, 400, 200, "");
    let mut play_btn = Button::new(160, 210, 80, 40, "play");
    let mut stop_btn = Button::new(160, 250, 80, 40, "stop");

    let mut seeker = Slider::new(10, 10, 300, 10, "slider");
    seeker.set_type(SliderType::Horizontal);
    seeker.set_frame(FrameType::RFlatBox);
    seeker.set_color(Color::from_u32(0x868db1));
    let seeker_tx = tx.clone();
    seeker.set_callback(move |s| {
        seeker_tx
            .send(PlayerMessage::Seek(s.value()))
            .expect("failed to send Seek message");
        fltk::app::redraw();
    });

    seeker.draw(|s| {
        draw::set_draw_color(Color::Blue);
        draw::draw_pie(
            s.x() - 10 + (s.w() as f64 * s.value()) as i32,
            s.y() - 10,
            30,
            30,
            0.,
            360.,
        );
    });

    let seeker_pos_tx = tx.clone();
    let (seeker_tx, seeker_rx) = channel::<f64>();
    tokio::spawn(async move {
        for _ in 0.. {
            let tx_clone = seeker_tx.clone();
            let msg: PlayerMessage = PlayerMessage::GetPos(Box::new(move |pos| {
                tx_clone
                    .send(pos)
                    .expect("failed to send position to seeker");
            }));
            seeker_pos_tx
                .send(msg)
                .expect("failed to send get_pos message");
            sleep(0.05);
        }
    });

    tokio::spawn(async move {
        for pos in seeker_rx {
            seeker.set_value(pos);
            fltk::app::awake();
            fltk::app::redraw();
        }
    });

    win.set_color(html::Red);
    play_btn.set_frame(FrameType::NoBox);
    play_btn.set_color(html::Red);
    stop_btn.set_frame(FrameType::NoBox);
    stop_btn.set_color(html::Red);
    win.end();
    win.show();
    let play_btn_tx = tx.clone();
    play_btn.set_callback(move |_| {
        frame.set_label("you clicked the button!");
        println!("clicked");
        play_btn_tx
            .send(PlayerMessage::Play)
            .expect("failed to send play message");
    });
    let stop_btn_tx = tx.clone();
    stop_btn.set_callback(move |_| {
        stop_btn_tx
            .send(PlayerMessage::Stop)
            .expect("failed to send stop message");
    });

    // kill all threads when the X button is clicked
    win.set_callback(move |_| {
        tx.send(PlayerMessage::Quit)
            .expect("failed to send quit message");
        app.quit();
    });
    app.run().expect("could not run app");
    Ok(())
}

async fn play_manager(rx: Receiver<PlayerMessage>) {
    // let stream_handle = OutputStream::try_default().unwrap();
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
            }
            PlayerMessage::Stop => {
                println!("stoped");
                sink.pause();
            }
            PlayerMessage::Seek(place) => {
                println!("seeking {}", place);
                let pos = place * duration.as_secs_f64();
                sink.try_seek(Duration::from_secs_f64(pos))
                    .expect("could not seek");
            }
            PlayerMessage::Quit => {
                return;
            }
            PlayerMessage::GetPos(mut call_back) => {
                let pos = sink.get_pos();
                let f64_pos = pos.as_secs_f64() / duration.as_secs_f64();
                call_back(f64_pos);
            }
        }
    }
}
