use std::path::{Path, PathBuf};
use std::{env, process};
use std::{fs, io};

use clap::{App, Arg};
use opml::{Outline, OPML};
use podchiver::Podcast;

mod podchiver;

fn main() {
    let matches = App::new(clap::crate_name!())
        .about(clap::crate_description!())
        .author(clap::crate_authors!())
        .version(clap::crate_version!())
        .arg(Arg::with_name("opml").long("opml").takes_value(true))
        .arg(Arg::with_name("rss").long("rss").takes_value(true))
        .arg(
            Arg::with_name("download-directory")
                .short("d")
                .long("download-directory")
                .takes_value(true),
        )
        .setting(clap::AppSettings::ArgRequiredElseHelp)
        .get_matches();

    let download_directory =
        if let Some(download_directory) = matches.value_of("download-directory") {
            PathBuf::from(download_directory)
        } else {
            std::env::current_dir().expect("Failed to get current directory")
        };

    if let Some(opml_path) = matches.value_of("opml") {
        match read_file(opml_path) {
            Ok(opml_file_contents) => {
                if let Ok(opml) = OPML::from_str(&opml_file_contents) {
                    for outline in opml.body.outlines {
                        let podcasts = process_outline(outline);
                        for podcast in podcasts {
                            download_episodes(&podcast, &download_directory);
                        }
                    }
                } else {
                    eprintln!("Failed to parse OPML file");
                }
            }
            Err(err) => eprintln!("Failed to read OPML file: {}", err),
        }
    }

    if let Some(rss_path) = matches.value_of("rss") {
        if let Ok(rss_file_contents) = read_file(rss_path) {
            let podcast = Podcast::new(&rss_file_contents).expect("Failed to parse RSS XML");
            download_episodes(&podcast, &download_directory);
        } else {
            eprintln!("Failed to read RSS file")
        }
    }
}

fn process_outline(outline: Outline) -> Vec<Podcast> {
    let mut podcasts = Vec::new();

    let podcast_title = outline.text;

    if let Some(url) = outline.xml_url {
        print!("Fetching {} ({})...", podcast_title, url);
        let response = ureq::get(&url).timeout_connect(6_000).call();
        println!(" {}", response.status_text());
        if response.ok() {
            if let Ok(response_body) = response.into_string() {
                if let Ok(podcast) = podchiver::Podcast::new(&response_body) {
                    podcasts.push(podcast);
                } else {
                    eprintln!("Failed to parse RSS XML");
                }
            }
        }
    }

    podcasts
}

fn download_episodes(podcast: &podchiver::Podcast, download_directory: &Path) {
    let mut podcast_download_directory = PathBuf::from(&download_directory);
    podcast_download_directory.push(&podcast.dir_name());

    create_directory(&podcast_download_directory);

    for episode in &podcast.episodes {
        let mut file_path = PathBuf::new();
        file_path.push(&podcast_download_directory);
        file_path.push(episode.filename());

        println!("Downloading {} to {}...", episode.url, file_path.display());

        let mut request = ureq::get(&episode.url).call().into_reader();
        let mut out = fs::File::create(file_path).expect("Failed to create file");
        if let Err(error) = io::copy(&mut request, &mut out) {
            eprint!("Failed to download podcast: {}", error.to_string());
        }
    }
}

fn create_directory(path: &Path) {
    if !path.exists() {
        if let Err(error) = fs::create_dir(path) {
            eprintln!(
                "Failed to create directory {}: {}",
                path.display(),
                error.to_string()
            );
            process::exit(1);
        }
        println!("Created {}...", path.display());
    } else if !path.is_dir() {
        eprintln!("{} exists but is not a directory", path.display());
        process::exit(2);
    }
}

/// Attempt to read and return the file at the specified location,
/// either via HTTP or the filesystem.
fn read_file(location: &str) -> io::Result<String> {
    if location.starts_with("http") || location.starts_with("https") {
        let request = ureq::get(location).timeout_connect(6_000).call();
        request.into_string()
    } else {
        fs::read_to_string(location)
    }
}
