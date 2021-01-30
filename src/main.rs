use std::fs;
use std::path::{Path, PathBuf};
use std::{env, process};

use opml::{Outline, OPML};
use rss::Channel;
use url::Url;

fn main() {
    let args = env::args();
    if args.count() < 2 {
        eprintln!("No OPML file specified");
        return;
    }

    let opml_file_path = env::args().nth(1).unwrap();

    if let Ok(opml_file_contents) = fs::read_to_string(&opml_file_path) {
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

fn process_outline(outline: Outline) {
    let podcast_title = outline.text;

    if let Some(url) = outline.xml_url {
        println!("Fetching {} ({})...", podcast_title, url);
        let response = ureq::get(&url).timeout_connect(6_000).call();
        if response.ok() {
            println!("Response: {}", response.status());
            if let Ok(response_body) = response.into_string() {
                if let Ok(channel) = Channel::read_from(response_body.as_bytes()) {
                    for item in channel.items() {
                        if let Some(episode_title) = item.title() {
                            if let Some(enclosure) = item.enclosure() {
                                download_episode(enclosure.url(), &episode_title, &podcast_title);
                            }
                        }
                    }
                }
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

            println!("{}", pathbuf.display());

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
