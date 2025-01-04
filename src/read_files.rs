use std::{fs, path::Path};

use lofty::{file::TaggedFileExt, read_from_path, tag::TagItem};

use crate::Song;

/// recursivly search a directory for sound files, parse, and return them.
pub fn search_dir<T: AsRef<Path>>(path: T) -> Vec<Song> {
    println!("searching {:?}", path.as_ref());
    let songs = fs::read_dir(path).expect("failed to read dir");
    let songs = songs
        .map(|song| {
            let song = song.expect("song is error");
            if song.file_type().expect("failed to get file type").is_dir() {
                search_dir(song.path())
            } else {
                let path = song.path();
                let tagged_file = match read_from_path(path.clone()) {
                    Ok(tf) => tf,
                    Err(_) => {
                        println!("error: cannot read tagged file {:?}", path);
                        return Vec::new();
                    }
                };
                let a = match tagged_file.primary_tag() {
                    Some(tf) => tf,
                    None => {
                        println!("error: no primary tag {:?}", path);
                        return Vec::new();
                    }
                };
                vec![a.items().fold(Song::new(path.clone()), fold_songs)]
            }
        })
        .flatten()
        .collect();
    songs
}

/// takes a `Song` and a `TagItem` addes the metadata in the TagItem to the song or just
/// returns if it isn't a attribute we care about.
/// intended to be use in a fold.
///
/// # Example
/// ```
/// let tagged_file = read_from_path("path to file").unwrap();
/// let primary_tag = tagged_file.primary_tag();
/// primary_tag
///     .unwrap()
///     .items()
///     .fold(Song::new("path to file"), fold_songs);
/// ```
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
        lofty::tag::ItemKey::DiscNumber => {
            song.disc_number = tag
                .into_value()
                .into_string()
                .map(|x| x.parse().expect("failed to parse value"))
        }
        lofty::tag::ItemKey::DiscTotal => {}
        lofty::tag::ItemKey::TrackNumber => {
            song.track_number = tag
                .into_value()
                .into_string()
                .map(|x| x.parse().expect("failed to parse value"))
        }
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
