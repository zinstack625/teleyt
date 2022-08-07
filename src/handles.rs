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

pub async fn user_config(
    api: AsyncApi,
    message: Message,
    config: crate::config::Config,
) -> Result<(), Error> {
    let user_config = db::get_config(*message.from.as_ref().unwrap().clone(), config.clone())
        .await
        .unwrap();
    let message_markup = frankenstein::InlineKeyboardMarkup::builder()
        .inline_keyboard(vec![vec![frankenstein::InlineKeyboardButton::builder()
            .text(match user_config.delete_on_send {
                Some(true) => "Don't delete",
                Some(false) => "Delete",
                None => "Don't delete",
            })
            .callback_data(match user_config.delete_on_send {
                Some(true) => "cfg_user_0",
                Some(false) => "cfg_user_1",
                None => "cfg_user_0",
            })
            .build()]])
        .build();
    let message = frankenstein::SendMessageParams::builder()
        .chat_id(message.chat.id)
        .text("Personal configuration")
        .reply_markup(frankenstein::ReplyMarkup::InlineKeyboardMarkup(
            message_markup,
        ))
        .build();
    tokio::spawn(async move {
        let _ = api.send_message(&message).await;
    });
    Ok(())
}

pub async fn group_config(
    api: AsyncApi,
    message: Message,
    config: crate::config::Config,
) -> Result<(), Error> {
    let group_config = db::get_config(*message.chat.clone(), config.clone())
        .await
        .unwrap();

    let message_markup = frankenstein::InlineKeyboardMarkup::builder()
        .inline_keyboard(vec![vec![frankenstein::InlineKeyboardButton::builder()
            .text(match group_config.delete_on_send {
                Some(true) => "Don't delete",
                Some(false) => "Delete",
                None => "Don't delete",
            })
            .callback_data(match group_config.delete_on_send {
                Some(true) => "Don't delete",
                Some(false) => "Delete",
                None => "Don't delete",
            })
            .build()]])
        .build();
    let message = frankenstein::SendMessageParams::builder()
        .chat_id(message.chat.id)
        .text("Group configuration")
        .reply_markup(frankenstein::ReplyMarkup::InlineKeyboardMarkup(
            message_markup,
        ))
        .build();
    tokio::spawn(async move {
        let _ = api.send_message(&message).await;
    });
    Ok(())
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
            Ok(None) => {
                let _ = tokio::time::sleep(std::time::Duration::from_millis(5));
                tokio::task::yield_now().await
            }
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

pub async fn set_status<T: 'static + db::HasID + Clone + Send>(
    api: AsyncApi,
    id: T,
    status: user_status::UserStatus,
    config: crate::config::Config,
) -> Result<(), crate::db::DbError> {
    println!(
        "Setting user {} status to {:?}",
        id.clone().get_id(),
        status
    );
    let status_fut = tokio::spawn(db::set_status(id.clone(), status, config.clone()));
    tokio::spawn(async move {
        let api = api.clone();
        let send_msg_params = SendMessageParams::builder()
            .chat_id(id.get_id() as i64)
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
    tokio::spawn(db::set_status(
        *message.from.as_ref().unwrap().clone(),
        user_status::UserStatus::None,
        config.clone(),
    ));
    let user_config_fut = tokio::spawn(db::get_config(
        *message.from.as_ref().unwrap().clone(),
        config.clone(),
    ));
    let group_config_fut = tokio::spawn(db::get_config(*message.chat.clone(), config.clone()));
    let link_name = link.to_string();
    let vid_name = tokio::spawn(async move { get_name(&link_name).await });
    let vid = dwnld_file(link, FileType::Video, config.clone());
    if let Ok((mut vid, _dir)) = vid.await {
        let new_vid_path = vid.parent().unwrap().join(
            vid_name
                .await
                .unwrap()
                .unwrap_or_else(|_| "unknown".to_string()),
        );
        if std::fs::rename(vid.clone(), new_vid_path.clone()).is_ok() {
            vid = new_vid_path;
        }
        let api_clone = api.clone();
        let message_clone = message.clone();
        tokio::spawn(async move {
            // maintain ownership of tempdir
            let _dir = _dir;
            let send_vid_params = SendVideoParams::builder()
                .chat_id(message_clone.chat.id)
                .video(vid)
                .build();
            if let Err(some) = api_clone.send_video(&send_vid_params).await {
                let error_msg_params = SendMessageParams::builder()
                    .chat_id(message_clone.chat.id)
                    .text("Something went wrong: ".to_string() + &some.to_string())
                    .build();
                let _ = api_clone.send_message(&error_msg_params).await;
            }
        });
        let user_config = user_config_fut.await.unwrap_or(Err(db::DbError::Error));
        let group_config = group_config_fut.await.unwrap_or(Err(db::DbError::Error));
        if user_config.is_ok() && user_config.unwrap().delete_on_send == Some(true)
            || group_config.is_ok() && group_config.unwrap().delete_on_send == Some(true)
        {
            tokio::spawn(async move {
                let delete_msg_params = DeleteMessageParams::builder()
                    .chat_id(message.chat.id)
                    .message_id(message.message_id)
                    .build();
                let _ = api.delete_message(&delete_msg_params).await;
            });
        }
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
    tokio::spawn(db::set_status(
        *message.chat.clone(),
        user_status::UserStatus::None,
        config.clone(),
    ));
    let user_config_fut = tokio::spawn(db::get_config(
        *message.from.as_ref().unwrap().clone(),
        config.clone(),
    ));
    let group_config_fut = tokio::spawn(db::get_config(*message.chat.clone(), config.clone()));
    let link_name = link.to_string();
    let mus_name = tokio::spawn(async move { get_name(&link_name).await });
    let mus = dwnld_file(link, FileType::Audio, config.clone());
    if let Ok((mut mus, _dir)) = mus.await {
        let new_mus_path = mus.parent().unwrap().join(
            mus_name
                .await
                .unwrap()
                .unwrap_or_else(|_| "unknown".to_string()),
        );
        if std::fs::rename(mus.clone(), new_mus_path.clone()).is_ok() {
            mus = new_mus_path;
        }
        let api_clone = api.clone();
        let message_clone = message.clone();
        tokio::spawn(async move {
            // maintain ownership of tempdir
            let _dir = _dir;
            let send_mus_params = SendAudioParams::builder()
                .chat_id(message_clone.chat.id)
                .audio(mus)
                .build();
            if let Err(some) = api_clone.send_audio(&send_mus_params).await {
                let error_msg_params = SendMessageParams::builder()
                    .chat_id(message_clone.chat.id)
                    .text("Something went wrong: ".to_string() + &some.to_string())
                    .build();
                let _ = api_clone.send_message(&error_msg_params).await;
            }
        });
        let user_config = user_config_fut.await.unwrap_or(Err(db::DbError::Error));
        let group_config = group_config_fut.await.unwrap_or(Err(db::DbError::Error));
        if user_config.is_ok() && user_config.unwrap().delete_on_send == Some(true)
            || group_config.is_ok() && group_config.unwrap().delete_on_send == Some(true)
        {
            tokio::spawn(async move {
                let delete_msg_params = DeleteMessageParams::builder()
                    .chat_id(message.chat.id)
                    .message_id(message.message_id)
                    .build();
                let _ = api.delete_message(&delete_msg_params).await;
            });
        }
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
