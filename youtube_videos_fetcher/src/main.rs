use reqwest::Error;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use dotenv::dotenv;
use std::env;

const CHANNEL_ID: &str = "UClmfDPjhb2vhfTmKAuoUO8A";  // Channel ID for JeaFxForexTrading
const BASE_URL_SEARCH: &str = "https://www.googleapis.com/youtube/v3/search";
const BASE_URL_VIDEOS: &str = "https://www.googleapis.com/youtube/v3/videos";
const VIDEO_URL_PREFIX: &str = "https://www.youtube.com/watch?v=";

#[derive(Debug, Serialize, Deserialize)]
struct Video {
    title: String,
    url: String,
    publish_date: String,
    length_minutes: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse {
    items: Vec<Item>,
    nextPageToken: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Item {
    id: Id,
    snippet: Snippet,
}

#[derive(Debug, Serialize, Deserialize)]
struct Id {
    kind: String,
    videoId: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Snippet {
    title: String,
    publishedAt: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct VideoDetailsResponse {
    items: Vec<VideoDetails>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VideoDetails {
    contentDetails: ContentDetails,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContentDetails {
    duration: String,  // The ISO 8601 formatted duration string
}

async fn fetch_video_details(api_key: &str, video_ids: &str) -> Result<Vec<String>, Error> {
    let url = format!(
        "{}?key={}&id={}&part=contentDetails",
        BASE_URL_VIDEOS, api_key, video_ids
    );

    let response: VideoDetailsResponse = reqwest::get(&url).await?.json().await?;
    let mut durations = Vec::new();

    for item in response.items {
        let duration_str = parse_iso8601_duration(&item.contentDetails.duration);
        durations.push(duration_str);
    }

    Ok(durations)
}


// Helper function to parse ISO 8601 duration into minutes
fn parse_iso8601_duration(duration: &str) -> String {
    let mut hours = 0;
    let mut minutes = 0;
    let mut seconds = 0;

    let re = regex::Regex::new(r"PT(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?").unwrap();
    if let Some(caps) = re.captures(duration) {
        if let Some(h) = caps.get(1) {
            hours = h.as_str().parse::<i64>().unwrap();
        }
        if let Some(m) = caps.get(2) {
            minutes = m.as_str().parse::<i64>().unwrap();
        }
        if let Some(s) = caps.get(3) {
            seconds = s.as_str().parse::<i64>().unwrap();
        }
    }

    // Convert hours to minutes
    if hours > 0 {
        minutes += hours * 60;
    }

    // Format the duration as "Xm Ys" or "Xs"
    if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}


async fn fetch_videos(api_key: &str, channel_id: &str) -> Result<Vec<Video>, Error> {
    let mut video_data = Vec::new();
    let mut next_page_token = None;

    loop {
        let mut url = format!(
            "{}?key={}&channelId={}&part=snippet&order=date&maxResults=50",
            BASE_URL_SEARCH, api_key, channel_id
        );

        if let Some(token) = &next_page_token {
            url.push_str(&format!("&pageToken={}", token));
        }

        let response: ApiResponse = reqwest::get(&url).await?.json().await?;
        let mut video_ids = Vec::new();

        // Collect video IDs for fetching durations
        for item in response.items.iter() {
            if item.id.kind == "youtube#video" {
                if let Some(video_id) = &item.id.videoId {
                    video_ids.push(video_id.clone());
                }
            }
        }

        // Fetch video details including duration
        if !video_ids.is_empty() {
            let video_ids_str = video_ids.join(",");
            let durations = fetch_video_details(api_key, &video_ids_str).await?;

            // Ensure we don't go out of bounds by limiting the iteration to the smallest vector size
            let min_len = std::cmp::min(response.items.len(), durations.len());

            for i in 0..min_len {
                let item = &response.items[i];
                if item.id.kind == "youtube#video" {
                    if let Some(video_id) = &item.id.videoId {
                        video_data.push(Video {
                            title: item.snippet.title.clone(),
                            url: format!("{}{}", VIDEO_URL_PREFIX, video_id),
                            publish_date: item.snippet.publishedAt.clone(),
                            length_minutes: durations[i].clone(),
                        });
                    }
                }
            }
        }

        next_page_token = response.nextPageToken;
        if next_page_token.is_none() {
            break;
        }
    }

    Ok(video_data)
}


fn save_to_json(videos: &[Video], filename: &str) {
    let file = File::create(filename).expect("Failed to create file");
    serde_json::to_writer_pretty(file, videos).expect("Failed to write JSON to file");
}

#[tokio::main]
async fn main() {
    dotenv().ok();  // Load environment variables from the .env file

    let api_key = env::var("YOUTUBE_API_KEY").expect("YOUTUBE_API_KEY not set in .env file");

    match fetch_videos(&api_key, CHANNEL_ID).await {
        Ok(videos) => {
            println!("Fetched {} videos", videos.len());
            save_to_json(&videos, "youtube_videos_with_durations.json");
            println!("Saved to youtube_videos_with_durations.json");
        }
        Err(e) => println!("Error fetching videos: {}", e),
    }
}
