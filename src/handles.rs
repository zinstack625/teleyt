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

async fn dwnld_file(
    link: &str,
    ftype: FileType,
    config: crate::config::Config,
) -> std::io::Result<(PathBuf, tempfile::TempDir)> {
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
        FileType::Video => &config.vid_format,
        FileType::Audio => &config.aud_format,
    };
    let mut proc = Command::new("yt-dlp")
        .args(["-f", format, "-o", &filename, "-v", link])
        .spawn()?;
    loop {
        match proc.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "Process failed to execute",
                    ));
                }
                break;
            }
            Ok(None) => tokio::task::yield_now().await,
            Err(a) => return Err(a),
        };
    }
    Ok((filepath, dir))
}

async fn get_name(url: &str) -> Result<String, std::io::Error> {
    let name = Command::new("yt-dlp")
        .args(["--skip-download", "--get-title", url])
        .output()
        .await?
        .stdout;
    let name = std::str::from_utf8(&name)
        .unwrap_or("unknown_name")
        .to_string();
    Ok(name)
}

pub async fn set_status(
    api: AsyncApi,
    id: Chat,
    status: user_status::UserStatus,
    _config: crate::config::Config,
) -> Result<(), crate::db::DbError> {
    println!("Setting user {} status to {:?}", id.id, status);
    let status_fut = tokio::spawn(db::set_user_status(id.clone(), status));
    tokio::spawn(async move {
        let api = api.clone();
        let send_msg_params = SendMessageParams::builder()
            .chat_id(id.id)
            .text("What to send?")
            .build();
        let _ = api.send_message(&send_msg_params).await;
    });
    status_fut.await.unwrap()?;
    Ok(())
}

pub async fn vid_handle(
    api: AsyncApi,
    message: Message,
    link: &str,
    config: crate::config::Config,
) -> Result<(), Error> {
    tokio::spawn(db::set_user_status(
        message.chat.clone(),
        user_status::UserStatus::None,
    ));
    let link_name = link.to_string();
    let vid_name = tokio::spawn(async move { get_name(&link_name).await });
    let vid = dwnld_file(link, FileType::Video, config.clone());
    if let Ok((mut vid, _dir)) = vid.await {
        let vid_name = vid_name
            .await
            .unwrap()
            .unwrap_or_else(|_| "unknown".to_string());
        if std::fs::rename(vid.clone(), vid_name.clone()).is_ok() {
            vid = vid.parent().unwrap().join(vid_name);
        }
        let api_clone = api.clone();
        tokio::spawn(async move {
            // maintain ownership of tempdir
            let _dir = _dir;
            let send_vid_params = SendVideoParams::builder()
                .chat_id(message.chat.id)
                .video(vid)
                .build();
            if let Err(some) = api_clone.send_video(&send_vid_params).await {
                let error_msg_params = SendMessageParams::builder()
                    .chat_id(message.chat.id)
                    .text("Something went wrong: ".to_string() + &some.to_string())
                    .build();
                let _ = api_clone.send_message(&error_msg_params).await;
            }
        });
        tokio::spawn(async move {
            let delete_msg_params = DeleteMessageParams::builder()
                .chat_id(message.chat.id)
                .message_id(message.message_id)
                .build();
            let _ = api.delete_message(&delete_msg_params).await;
        });
    } else {
        tokio::spawn(async move {
            let error_msg_params = SendMessageParams::builder()
                .chat_id(message.chat.id)
                .text("Something went wrong")
                .build();
            let _ = api.send_message(&error_msg_params).await;
        });
    }
    Ok(())
}

pub async fn mus_handle(
    api: AsyncApi,
    message: Message,
    link: &str,
    config: crate::config::Config,
) -> Result<(), Error> {
    tokio::spawn(db::set_user_status(
        message.chat.clone(),
        user_status::UserStatus::None,
    ));
    let link_name = link.to_string();
    let mus_name = tokio::spawn(async move { get_name(&link_name).await });
    let mus = dwnld_file(link, FileType::Audio, config.clone());
    if let Ok((mut mus, _dir)) = mus.await {
        let mus_name = mus_name
            .await
            .unwrap()
            .unwrap_or_else(|_| "unknown".to_string());
        let mus_name = mus.parent().unwrap().join(mus_name);
        if std::fs::rename(mus.clone(), mus_name.clone()).is_ok() {
            mus = mus.parent().unwrap().join(mus_name);
        }
        let api_clone = api.clone();
        tokio::spawn(async move {
            // maintain ownership of tempdir
            let _dir = _dir;
            let send_mus_params = SendAudioParams::builder()
                .chat_id(message.chat.id)
                .audio(mus)
                .build();
            if let Err(some) = api_clone.send_audio(&send_mus_params).await {
                let error_msg_params = SendMessageParams::builder()
                    .chat_id(message.chat.id)
                    .text("Something went wrong: ".to_string() + &some.to_string())
                    .build();
                let _ = api_clone.send_message(&error_msg_params).await;
            }
        });
        tokio::spawn(async move {
            let delete_msg_params = DeleteMessageParams::builder()
                .chat_id(message.chat.id)
                .message_id(message.message_id)
                .build();
            let _ = api.delete_message(&delete_msg_params).await;
        });
    } else {
        tokio::spawn(async move {
            let error_msg_params = SendMessageParams::builder()
                .chat_id(message.chat.id)
                .text("Something went wrong")
                .build();
            let _ = api.send_message(&error_msg_params).await;
        });
    }
    Ok(())
}
