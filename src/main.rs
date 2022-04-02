use futures::StreamExt;
use telegram_bot::*;
use user_status::UserStatus;

mod db;
mod handles;
mod user_status;

async fn match_handles(api: Api, message: Message) -> Result<(), Error> {
    if let MessageKind::Text { ref data, .. } = message.kind {
        let msg_text = data.as_str();
        if let Ok(status) = db::get_user_status(message.chat.id()).await {
            match status {
                UserStatus::MusRequest => {
                    handles::mus_handle(api, message.clone(), msg_text).await?;
                    return Ok(());
                }
                UserStatus::VidRequest => {
                    handles::vid_handle(api, message.clone(), msg_text).await?;
                    return Ok(());
                }
                UserStatus::None => {}
            }
        }
        if msg_text.len() > 4 {
            if msg_text.starts_with("/vid") {
                handles::vid_handle(api, message.clone(), &msg_text[4..]).await?
            } else if msg_text.starts_with("/mus") {
                handles::mus_handle(api, message.clone(), &msg_text[4..]).await?
            }
        } else {
            empty_status_handle(api, message.clone(), msg_text).await?;
        }
    }
    Ok(())
}

async fn empty_status_handle(api: Api, message: Message, text: &str) -> Result<(), Error> {
    if let MessageKind::Text { ref data, .. } = message.kind {
        if text.starts_with("/vid") {
            handles::vid_empty_handle(api.clone(), message.clone()).await;
        } else if text.starts_with("/mus") {
            handles::mus_empty_handle(api.clone(), message.clone()).await;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let token = std::env::var("TELEGRAM_BOT_TOKEN").expect("Set TELEGRAM_BOT_TOKEN envvar");
    let api = Api::new(token);

    let mut stream = api.stream();
    stream.error_delay(std::time::Duration::from_secs(5u64));
    while let Some(update) = stream.next().await {
        if let Ok(update) = update {
            if let UpdateKind::Message(message) = update.kind {
                let api = api.clone();
                tokio::spawn(async move {
                    if let Err(error) = match_handles(api.clone(), message.clone()).await {
                        println!("{}", error);
                        api.send(message.chat.text(error.to_string())).await;
                    }
                });
            }
        }
    }
    Ok(())
}
