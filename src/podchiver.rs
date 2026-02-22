use std::path::PathBuf;

use chrono::{DateTime, FixedOffset};

pub struct Episode {
    pub title: Option<String>,
    pub url: String,
    pub pub_date: Option<DateTime<FixedOffset>>,
}

pub struct Podcast {
    pub title: String,
    pub episodes: Vec<Episode>,
}

impl Podcast {
    pub fn new(xml: &str) -> Result<Podcast, rss::Error> {
        let channel = rss::Channel::read_from(xml.as_bytes())?;

        let mut episodes: Vec<Episode> = Vec::new();

        for item in channel.items() {
            let date_time = item
                .pub_date
                .as_deref()
                .and_then(|d| DateTime::parse_from_rfc2822(d).ok());

            if let Some(enclose) = item.enclosure() {
                episodes.push(Episode {
                    title: item.title.clone(),
                    url: enclose.url.clone(),
                    pub_date: date_time,
                })
            }
        }

        Ok(Podcast {
            title: channel.title,
            episodes,
        })
    }

    pub fn dir_name(&self) -> PathBuf {
        PathBuf::from(sanitize_string(&self.title))
    }
}

impl Episode {
    pub fn filename(&self, no_date: bool) -> PathBuf {
        if let Ok(parsed_url) = url::Url::parse(&self.url)
            && let Some(file_extension) = parsed_url.path().split('.').next_back()
            && let Some(title) = &self.title
        {
            let mut pathbuf = PathBuf::new();

            let filename = if no_date && let Some(pub_date) = &self.pub_date {
                let date_prefix = pub_date.format("%Y%m%d-%H%M%SZ");
                format!(
                    "{}_{}.{}",
                    date_prefix,
                    sanitize_string(title),
                    file_extension
                )
            } else {
                format!("{}.{}", sanitize_string(title), file_extension)
            };
            pathbuf.push(filename);

            return pathbuf;
        }

        // FIXME: This is a terrible default.
        PathBuf::from("out")
    }
}

/// Creates a new string from the supplied string, but with all of the
/// characters that are illegal in Windows and Linux paths removed.
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
