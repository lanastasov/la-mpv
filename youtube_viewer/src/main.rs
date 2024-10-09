use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::fs;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_files::Files;
use serde_json::json;
use rusqlite::{params, Connection, Result};
use std::process::Command;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Video {
    title: String,
    url: String,
    publish_date: DateTime<Utc>,
    length_minutes: f64,
}

fn read_videos() -> Vec<Video> {
    let data = fs::read_to_string("../OwenThurm.json").expect("Unable to read file");
    let videos: Vec<Video> = serde_json::from_str(&data).expect("JSON was not well-formatted");
    videos
}

fn sort_by_publish_date(videos: &mut Vec<Video>) {
    videos.sort_by(|a, b| b.publish_date.cmp(&a.publish_date));
}

fn sort_by_length_minutes(videos: &mut Vec<Video>) {
    videos.sort_by(|a, b| a.length_minutes.partial_cmp(&b.length_minutes).unwrap());
}

fn sort_by_title(videos: &mut Vec<Video>) {
    videos.sort_by(|a, b| a.title.cmp(&b.title));
}

async fn index(sort_by: web::Path<String>) -> impl Responder {
    let mut videos = read_videos();

    match sort_by.as_str() {
        "publish_date" => sort_by_publish_date(&mut videos),
        "length_minutes" => sort_by_length_minutes(&mut videos),
        "title" => sort_by_title(&mut videos),
        _ => (),
    }

    let json_response = json!(videos);
    HttpResponse::Ok().json(json_response)
}

fn init_db() -> Result<Connection> {
    let conn = Connection::open("watched_videos.db")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS watched_videos (
            url TEXT PRIMARY KEY
        )",
        params![],
    )?;

    Ok(conn)
}

async fn watch_video(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    if let Some(url) = query.get("url") {
        let conn = init_db().expect("Failed to initialize database");
        conn.execute(
            "INSERT INTO watched_videos (url) VALUES (?1)",
            params![url],
        ).expect("Failed to insert watched video");

        // Convert the URL to the short form
        let short_url = url.replace("https://www.youtube.com/watch?v=", "https://youtu.be/");

        // Execute the mpv command
        let status = Command::new("mpv")
            .arg(short_url)
            .arg("--speed=1.5")
            .status()
            .expect("Failed to execute mpv command");

        if status.success() {
            HttpResponse::Ok().body("Video marked as watched and opened with mpv")
        } else {
            HttpResponse::InternalServerError().body("Failed to open video with mpv")
        }
    } else {
        HttpResponse::BadRequest().body("Missing url parameter")
    }
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/sort/{sort_by}", web::get().to(index))
            .route("/watch", web::post().to(watch_video))
            .service(Files::new("/", "static").index_file("index.html"))

    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}