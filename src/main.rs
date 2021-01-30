use std::fs;
use std::path::{Path, PathBuf};
use std::{env, process};

use clap::{App, Arg};
use opml::{Outline, OPML};
use rss::Channel;
use url::Url;

fn main() {
    let matches = App::new(clap::crate_name!())
        .about(clap::crate_description!())
        .author(clap::crate_authors!())
        .version(clap::crate_version!())
        .arg(Arg::with_name("opml").long("opml").takes_value(true))
        .arg(Arg::with_name("rss").long("rss").takes_value(true))
        .setting(clap::AppSettings::ArgRequiredElseHelp)
        .get_matches();

    if let Some(opml_path) = matches.value_of("opml") {
        if let Ok(opml_file_contents) = fs::read_to_string(opml_path) {
            if let Ok(opml) = OPML::new(&opml_file_contents) {
                for outline in opml.body.outlines {
                    process_outline(outline);
                }
            } else {
                eprintln!("Failed to parse OPML file");
            }
        } else {
            eprintln!("Failed to read OPML file");
        }
    }

    if let Some(rss_path) = matches.value_of("rss") {
        if let Ok(rss_file_contents) = fs::read_to_string(rss_path) {
            if let Ok(channel) = Channel::read_from(rss_file_contents.as_bytes()) {
                process_channel(channel);
            }
        }
    }
}

fn process_outline(outline: Outline) {
    let podcast_title = outline.text;

    if let Some(url) = outline.xml_url {
        print!("Fetching {} ({})...", podcast_title, url);
        let response = ureq::get(&url).timeout_connect(6_000).call();
        println!(" {}", response.status_text());
        if response.ok() {
            if let Ok(response_body) = response.into_string() {
                if let Ok(channel) = Channel::read_from(response_body.as_bytes()) {
                    process_channel(channel);
                }
            }
        }
    }
}

fn process_channel(channel: Channel) {
    for item in channel.items() {
        if let Some(episode_title) = item.title() {
            if let Some(enclosure) = item.enclosure() {
                download_episode(enclosure.url(), episode_title, &channel.title);
            }
        }
    }
}

fn download_episode(url: &str, episode_title: &str, podcast_title: &str) {
    let sanitized_episode_title = sanitize_string(episode_title);
    let sanitized_podcast_title = sanitize_string(podcast_title);

    create_directory(&sanitized_podcast_title);

    if let Ok(parsed_url) = Url::parse(url) {
        if let Some(file_extension) = parsed_url.path().split('.').last() {
            let mut pathbuf = PathBuf::new();
            pathbuf.push(sanitized_podcast_title);

            let filename = format!("{}.{}", sanitized_episode_title, file_extension);
            pathbuf.push(filename);

            if pathbuf.exists() {
                eprintln!("{} already exists", pathbuf.display());
            }

            println!("Downloading {} to {}...", url, pathbuf.display());

            let mut request = ureq::get(&url).call().into_reader();
            let mut out = std::fs::File::create(&pathbuf).expect("Failed to create file");
            if let Err(error) = std::io::copy(&mut request, &mut out) {
                eprint!("Failed to download podcast: {}", error.to_string());
            }
        } else {
            eprintln!("Failed to extract filename from URL");
        }
    } else {
        eprintln!("Failed to parse URL: {}", url)
    }
}

fn create_directory(name: &str) {
    let path = Path::new(name);
    if !path.exists() {
        if let Err(error) = fs::create_dir(name) {
            eprintln!("Failed to create directory {}: {}", name, error.to_string());
            process::exit(1);
        }
        println!("Created {}...", name);
    } else if !path.is_dir() {
        eprintln!("{} exists but is not a directory", name);
        process::exit(2);
    }
}

fn sanitize_string(s: &str) -> String {
    const UNSAFE_CHARS: [char; 9] = ['\\', '/', ':', '*', '?', '\"', '<', '>', '|'];

    let mut decoded_string = String::new();

    html_escape::decode_html_entities_to_string(s, &mut decoded_string);

    let sanitized_string: String = decoded_string
        .chars()
        .filter(|c| c.is_ascii() && !UNSAFE_CHARS.contains(c))
        .collect();

    sanitized_string
}
