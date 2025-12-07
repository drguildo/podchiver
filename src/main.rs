use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{env, process};

use clap::{Arg, Command};
use opml::{OPML, Outline};
use podchiver::Podcast;

use crate::progress_indicator::ProgressIndicator;

mod podchiver;
mod progress_indicator;

fn main() {
    let matches = Command::new(clap::crate_name!())
        .about(clap::crate_description!())
        .author(clap::crate_authors!())
        .version(clap::crate_version!())
        .arg(Arg::new("opml").long("opml"))
        .arg(Arg::new("rss").long("rss"))
        .arg(
            Arg::new("download-directory")
                .short('d')
                .long("download-directory"),
        )
        .arg_required_else_help(true)
        .get_matches();

    // Use the download directory specified on the command line, or the current
    // working directory if none was specified.
    let download_directory =
        if let Some(download_directory) = matches.get_one::<String>("download-directory") {
            PathBuf::from(download_directory)
        } else {
            std::env::current_dir().expect("Failed to get current directory")
        };

    // Read the OPML file specified on the command line.
    if let Some(opml_path) = matches.get_one::<String>("opml") {
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

    // Read the RSS file specified on the command line.
    if let Some(rss_path) = matches.get_one::<String>("rss") {
        if let Ok(rss_file_contents) = read_file(rss_path) {
            let podcast = Podcast::new(&rss_file_contents).expect("Failed to parse RSS XML");
            download_episodes(&podcast, &download_directory);
        } else {
            eprintln!("Failed to read RSS file")
        }
    }
}

/// Process a single OPML outline element, returning a vector of Podcasts.
fn process_outline(outline: Outline) -> Vec<Podcast> {
    let mut podcasts = Vec::new();

    let podcast_title = outline.text;

    // Extract the podcast's RSS file URL from the outline element.
    if let Some(url) = outline.xml_url {
        print!("Fetching {} ({})...", podcast_title, url);
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::new(6_000, 0)))
            .build();
        let agent: ureq::Agent = config.into();
        // Fetch the podcast's RSS file.
        if let Ok(mut response) = agent.get(&url).call() {
            println!(" {}", response.status());
            if let Ok(response_body) = response.body_mut().read_to_string() {
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
    podcast_download_directory.push(podcast.dir_name());

    if let Err(error) = fs::create_dir(&podcast_download_directory) {
        eprintln!(
            "Failed to create directory {}: {}",
            podcast_download_directory.display(),
            error
        );
        process::exit(1);
    }
    println!("Created {}...", podcast_download_directory.display());

    let config = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::new(6_000, 0)))
        .build();
    let agent: ureq::Agent = config.into();

    for episode in &podcast.episodes {
        let mut file_path = PathBuf::new();
        file_path.push(&podcast_download_directory);
        let filename = episode.filename();
        file_path.push(episode.filename());

        println!("Downloading {} to {}...", episode.url, file_path.display());

        if let Ok(mut response) = agent.get(&episode.url).call() {
            let total_size = response
                .headers()
                .get("Content-Length")
                .and_then(|val| val.to_str().ok()?.parse::<u64>().ok());
            // Initialize the progress indicator if the Content-Length header
            // was present.
            let mut progress_indicator = total_size.map(|total| ProgressIndicator::new(total, 72));
            let mut reader = response.body_mut().as_reader();
            let mut out_file = fs::File::create(file_path).expect("Failed to create file");
            let mut buffer = [0u8; 8192];
            let mut bytes_downloaded: u64 = 0;

            loop {
                if let Ok(bytes_read) = reader.read(&mut buffer) {
                    if bytes_read == 0 {
                        // We've reached the end of the file.
                        break;
                    }

                    if out_file.write_all(&buffer[..bytes_read]).is_err() {
                        eprintln!("Failed to write buffer");
                        break;
                    }
                    bytes_downloaded += bytes_read as u64;

                    // Update the progress indicator.
                    if let Some(progress_indicator) = &mut progress_indicator {
                        progress_indicator.progress(bytes_read as u64);
                        progress_indicator.draw();
                    } else {
                        print!("\r{}: {} bytes", filename.display(), bytes_downloaded);
                    }
                } else {
                    eprintln!("Failed to read HTTP data")
                }
            }
        } else {
            eprintln!("Download HTTP request failed")
        }
        println!();
    }
}

/// Attempt to read and return the file at the specified location, either via
/// HTTP or the filesystem.
fn read_file(location: &str) -> io::Result<String> {
    let config = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::new(6_000, 0)))
        .build();
    let agent: ureq::Agent = config.into();

    if location.starts_with("http") || location.starts_with("https") {
        let mut response = agent
            .get(location)
            .call()
            .map_err(|e| io::Error::other(format!("HTTP error: {}", e)))?;
        response
            .body_mut()
            .read_to_string()
            .map_err(|e| io::Error::other(format!("Read error: {}", e)))
    } else {
        fs::read_to_string(location)
    }
}
