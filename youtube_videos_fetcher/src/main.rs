use reqwest::Error;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use dotenv::dotenv;
use std::env;

const CHANNEL_ID: &str = "UC1Jp-8YnEz3pjPkhCC555gA";  // Channel ID for JeaFxForexTrading
const BASE_URL: &str = "https://www.googleapis.com/youtube/v3/search";
const VIDEO_URL_PREFIX: &str = "https://www.youtube.com/watch?v=";

#[derive(Debug, Serialize, Deserialize)]
struct Video {
    title: String,
    url: String,
    publish_date: String,
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
    publishedAt: String,  // The field for the video publish date
}

async fn fetch_videos(api_key: &str, channel_id: &str) -> Result<Vec<Video>, Error> {
    let mut video_data = Vec::new();
    let mut next_page_token = None;

    loop {
        let mut url = format!(
            "{}?key={}&channelId={}&part=snippet&order=date&maxResults=50",
            BASE_URL, api_key, channel_id
        );

        if let Some(token) = &next_page_token {
            url.push_str(&format!("&pageToken={}", token));
        }

        let response = reqwest::get(&url).await?.text().await?;
        let parsed_response: ApiResponse = serde_json::from_str(&response).expect("Error parsing API response");

        for item in parsed_response.items {
            if item.id.kind == "youtube#video" {
                if let Some(video_id) = item.id.videoId {
                    video_data.push(Video {
                        title: item.snippet.title,
                        url: format!("{}{}", VIDEO_URL_PREFIX, video_id),
                        publish_date: item.snippet.publishedAt,  // Capture the publish date
                    });
                }
            }
        }

        next_page_token = parsed_response.nextPageToken;
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

    let api_key = env::var("YOUTUBE_API_KEY").expect("YOUTUBE_API_KEY not set in .env file");  // Read API key from env

    match fetch_videos(&api_key, CHANNEL_ID).await {
        Ok(videos) => {
            println!("Fetched {} videos", videos.len());
            save_to_json(&videos, "youtube_videos.json");
            println!("Saved to youtube_videos.json");
        }
        Err(e) => println!("Error fetching videos: {}", e),
    }
}
