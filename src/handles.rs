use std::process::Command;
use telegram_bot::prelude::*;
use telegram_bot::*;

use crate::db;
use crate::user_status;

fn get_name(url: &str) -> Result<String, std::io::Error> {
    let name = Command::new("yt-dlp")
        .args(["--skip-download", "--get-title", &url])
        .output()?
        .stdout;
    let name = std::str::from_utf8(&name)
        .unwrap_or_else(|_| "unknown_name")
        .to_string();
    Ok(name)
}

pub async fn vid_empty_handle(api: Api, message: Message) -> Result<(), crate::db::DbError> {
    db::set_user_status(message.chat.id(), user_status::UserStatus::VidRequest).await?;
    api.send(message.text_reply("What to send?")).await;
    Ok(())
}

pub async fn vid_handle(api: Api, message: Message, link: &str) -> Result<(), Error> {
    let mut vid_name = get_name(&link).unwrap_or_else(|_| "unknown_name".to_string());
    vid_name.push_str(".mp4");
    let vid = Command::new("yt-dlp")
        .args([
            "-f",
            "b[filesize_approx<=50m]/bv+ba[filesize_approx<=50m]",
            "-o",
            "-",
            &link,
        ])
        .output();
    if let Ok(vid) = vid {
        if vid.stdout.is_empty() {
            api.send(message.text_reply("Some error occured")).await?;
        }
        let vid = InputFileUpload::with_data(vid.stdout, vid_name);
        api.send(message.chat.video(vid)).await?;
        api.send(message.delete()).await?;
    } else {
        api.send(message.text_reply("Something went wrong!"))
            .await?;
    }
    Ok(())
}

pub async fn mus_empty_handle(api: Api, message: Message) -> Result<(), crate::db::DbError> {
    db::set_user_status(message.chat.id(), user_status::UserStatus::MusRequest).await?;
    api.send(message.text_reply("What to send?")).await;
    Ok(())
}

pub async fn mus_handle(api: Api, message: Message, link: &str) -> Result<(), Error> {
    let mut vid_name = get_name(&link).unwrap_or_else(|_| "unknown_name".to_string());
    vid_name.push_str(".mp3");
    let aud = Command::new("yt-dlp")
        .args([
            "-f",
            "ba[ext=m4a][filesize<=50m]/ba[ext=m4a][filesize_approx<=50m]",
            "-o",
            "-",
            &link,
        ])
        .output();
    if let Ok(aud) = aud {
        if aud.stdout.is_empty() {
            api.send(message.text_reply("Some error occured")).await?;
        }
        let aud = InputFileUpload::with_data(aud.stdout, vid_name);
        api.send(message.chat.audio(aud)).await?;
        api.send(message.delete()).await?;
    } else {
        api.send(message.text_reply("Something went wrong!"))
            .await?;
    }
    Ok(())
}
