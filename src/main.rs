use frankenstein::*;
use user_status::UserStatus;

mod db;
mod handles;
mod user_status;

async fn match_handles(api: AsyncApi, message: Message) -> Result<(), Error> {
    if let Some(msg_text) = message.text.clone() {
        if let Ok(status) = db::get_user_status(message.chat.clone()).await {
            match status {
                UserStatus::MusRequest => {
                    handles::mus_handle(api, message.clone(), &msg_text).await?;
                    return Ok(());
                }
                UserStatus::VidRequest => {
                    handles::vid_handle(api, message.clone(), &msg_text).await?;
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
            if msg_text.starts_with("/vid") {
                handles::set_status(api, message.chat, UserStatus::VidRequest).await;
            } else if msg_text.starts_with("/mus") {
                handles::set_status(api, message.chat, UserStatus::MusRequest).await;
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let token = std::env::var("TELEGRAM_BOT_TOKEN").expect("Set TELEGRAM_BOT_TOKEN envvar");

    let api = AsyncApi::new(&token);
    let update_params_builder =
        GetUpdatesParams::builder().allowed_updates(vec!["message".to_string()]);

    let mut update_params = update_params_builder.clone().build();

    loop {
        let results = api.get_updates(&update_params).await;
        match results {
            Ok(response) => {
                for update in response.result {
                    if let Some(message) = update.message {
                        let api_clone = api.clone();
                        tokio::spawn(async move {
                            if let Err(error) =
                                match_handles(api_clone.clone(), message.clone()).await
                            {
                                println!("{}", error);
                                let error_params = SendMessageParams::builder()
                                    .chat_id(message.chat.id)
                                    .text(error.to_string())
                                    .build();
                                api_clone.send_message(&error_params).await;
                            }
                        });
                        update_params = update_params_builder
                            .clone()
                            .offset(update.update_id + 1)
                            .build();
                    }
                }
            }
            Err(error) => {
                println!("Failed to get updates! {:?}", error);
            }
        }
    }
}
