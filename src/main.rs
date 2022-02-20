use futures::StreamExt;
use telegram_bot::prelude::*;
use telegram_bot::*;
use std::process::*;

async fn vid_handle(api: Api, message: Message) -> Result<(), Error> {
    let link = &message.text().unwrap()[5..];
    let vid = Command::new("youtube-dl")
        .args(["-o", "-", &link])
        .output();
    if let Ok(vid) = vid {
        let vid = InputFileUpload::with_data(vid.stdout, "vid.mp4");
        api.send(message.chat.video(vid)).await?;
        api.send(message.delete()).await?;
    } else {
        api.send(message.text_reply("Something went wrong!")).await?;
    }
    Ok(())
}

async fn mus_handle(api: Api, message: Message) -> Result<(), Error> {
    let link = &message.text().unwrap()[5..];
    let aud = Command::new("youtube-dl")
        .args(["-f", "bestaudio[ext=m4a]", "-o", "-", &link])
        .output();
    if let Ok(aud) = aud {
        let aud = InputFileUpload::with_data(aud.stdout, "aud.mp3");
        api.send(message.chat.audio(aud)).await?;
        api.send(message.delete()).await?;
    } else {
        api.send(message.text_reply("Something went wrong!")).await?;
    }
    Ok(())
}

async fn match_handles(api: Api, message: Message) -> Result<(), Error> {
    if let MessageKind::Text { ref data, .. } = message.kind {
        let msg_text = data.as_str();
        if msg_text.starts_with("/vid") {
            vid_handle(api.clone(), message).await?;
        } else if msg_text.starts_with("/mus") {
            mus_handle(api.clone(), message).await?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let token = std::env::var("TELEGRAM_BOT_TOKEN").expect("Set TELEGRAM_BOT_TOKEN envvar");
    let api = Api::new(token);

    let mut stream = api.stream();
    while let Some(update) = stream.next().await {
        let update = update?;
        if let UpdateKind::Message(message) = update.kind {
            match_handles(api.clone(), message).await?;
        }
    }
    Ok(())
}
