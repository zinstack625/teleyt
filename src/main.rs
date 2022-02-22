use futures::StreamExt;
use telegram_bot::*;

pub mod handles;

async fn match_handles(api: Api, message: Message) -> Result<(), Error> {
    if let MessageKind::Text { ref data, .. } = message.kind {
        let msg_text = data.as_str();
        if msg_text.starts_with("/vid") {
            handles::vid_handle(api.clone(), message).await?;
        } else if msg_text.starts_with("/mus") {
            handles::mus_handle(api.clone(), message).await?;
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
