use frankenstein::*;
use std::path::PathBuf;
use tempfile::tempdir;
use tokio::process::Command;

use crate::db;
use crate::user_status;

enum FileType {
    Video,
    Audio,
}

async fn dwnld_file(link: &str, ftype: FileType) -> std::io::Result<(PathBuf, tempfile::TempDir)> {
    let dir = tempdir()?;
    let name = String::from("file");
    let filepath = dir.path().join(name);
    let filename = filepath.to_str();
    if filename.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Unable to get filepath for youtube-dl",
        ));
    }
    let filename = filename.unwrap().to_string();
    let format = match ftype {
        Video => "b[filesize<=50m]/b[filesize_approx<=50m]/bv+ba[filesize_approx<=50m]",
        Audio => "ba[ext=m4a][filesize<=50m]/ba[ext=m4a][filesize_approx<=50m]",
    };
    Command::new("yt-dlp")
        .args(["-f", format, "-o", &filename, "-v", &link])
        .status()
        .await?;
    Ok((filepath, dir))
}

async fn get_name(url: &str) -> Result<String, std::io::Error> {
    let name = Command::new("yt-dlp")
        .args(["--skip-download", "--get-title", &url])
        .output()
        .await?
        .stdout;
    let name = std::str::from_utf8(&name)
        .unwrap_or_else(|_| "unknown_name")
        .to_string();
    Ok(name)
}

pub async fn set_status(
    api: AsyncApi,
    id: Chat,
    status: user_status::UserStatus,
) -> Result<(), crate::db::DbError> {
    println!("Setting user {} status to {:?}", id.id, status);
    let status_fut = tokio::spawn(db::set_user_status(id.clone(), status));
    tokio::spawn(async move {
        let api = api.clone();
        let send_msg_params = SendMessageParams::builder()
            .chat_id(id.id)
            .text("What to send?")
            .build();
        api.send_message(&send_msg_params).await;
    });
    status_fut.await.unwrap()?;
    Ok(())
}

pub async fn vid_handle(api: AsyncApi, message: Message, link: &str) -> Result<(), Error> {
    let link_name = link.to_string();
    let vid_name = tokio::spawn(async move { get_name(&link_name).await });
    let vid = dwnld_file(link, FileType::Video);
    if let Ok((vid, _dir)) = vid.await {
        let mut vid_name = vid_name.await.unwrap().unwrap_or("unknown".to_string());
        let vid_name = vid.parent().unwrap().join(vid_name);
        std::fs::rename(vid, vid_name.clone());
        let api_clone = api.clone();
        tokio::spawn(async move {
            // maintain ownership of tempdir
            let _dir = _dir;
            let send_vid_params = SendVideoParams::builder()
                .chat_id(message.chat.id)
                .video(vid_name)
                .build();
            api_clone.send_video(&send_vid_params).await;
        });
        tokio::spawn(async move {
            let delete_msg_params = DeleteMessageParams::builder()
                .chat_id(message.chat.id)
                .message_id(message.message_id)
                .build();
            api.delete_message(&delete_msg_params).await;
        });
    } else {
        tokio::spawn(async move {
            let error_msg_params = SendMessageParams::builder()
                .chat_id(message.chat.id)
                .text("Something went wrong")
                .build();
            api.send_message(&error_msg_params).await;
        });
    }
    tokio::spawn(db::set_user_status(
        message.chat,
        user_status::UserStatus::None,
    ));
    Ok(())
}

pub async fn mus_handle(api: AsyncApi, message: Message, link: &str) -> Result<(), Error> {
    let link_name = link.to_string();
    let mus_name = tokio::spawn(async move { get_name(&link_name).await });
    let mus = dwnld_file(link, FileType::Audio);
    if let Ok((mus, _dir)) = mus.await {
        let mut mus_name = mus_name.await.unwrap().unwrap_or("unknown".to_string());
        let mus_name = mus.parent().unwrap().join(mus_name);
        std::fs::rename(mus, mus_name.clone());
        let api_clone = api.clone();
        tokio::spawn(async move {
            // maintain ownership of tempdir
            let _dir = _dir;
            let send_mus_params = SendAudioParams::builder()
                .chat_id(message.chat.id)
                .audio(mus_name)
                .build();
            api_clone.send_audio(&send_mus_params).await;
        });
        tokio::spawn(async move {
            let delete_msg_params = DeleteMessageParams::builder()
                .chat_id(message.chat.id)
                .message_id(message.message_id)
                .build();
            api.delete_message(&delete_msg_params).await;
        });
    } else {
        tokio::spawn(async move {
            let error_msg_params = SendMessageParams::builder()
                .chat_id(message.chat.id)
                .text("Something went wrong")
                .build();
            api.send_message(&error_msg_params).await;
        });
    }
    tokio::spawn(db::set_user_status(
        message.chat,
        user_status::UserStatus::None,
    ));
    Ok(())
}
