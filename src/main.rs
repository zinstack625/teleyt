use frankenstein::*;
use std::fs::File;
use std::io::prelude::*;
use user_status::UserStatus;

mod config;
mod db;
mod handles;
mod user_status;

fn known_sites(link: &str) -> bool {
    link.starts_with("https://youtu.be") || link.starts_with("https://vk.com/video")
}

async fn match_handles(
    api: AsyncApi,
    message: Message,
    config: config::Config,
) -> Result<(), Error> {
    if let Some(msg_text) = message.text.clone() {
        if let Ok(status) = db::get_user_status(message.chat.clone(), config.clone()).await {
            match status {
                UserStatus::MusRequest => {
                    handles::mus_handle(api, message, &msg_text, config).await?;
                    return Ok(());
                }
                UserStatus::VidRequest => {
                    handles::vid_handle(api, message, &msg_text, config).await?;
                    return Ok(());
                }
                UserStatus::None => {}
            }
        }
        if msg_text.len() > 4 {
            if let Some(text) = msg_text.strip_prefix("/vid") {
                handles::vid_handle(api, message, text, config).await?
            } else if let Some(text) = msg_text.strip_prefix("/mus") {
                handles::mus_handle(api, message, text, config).await?
            } else if known_sites(&msg_text) {
                let _ = handles::vid_handle(api, message, &msg_text, config).await?;
            }
            return Ok(());
        } else if msg_text.starts_with("/vid") {
            let _ = handles::set_status(api, message.chat, UserStatus::VidRequest, config).await;
            return Ok(());
        } else if msg_text.starts_with("/mus") {
            let _ = handles::set_status(api, message.chat, UserStatus::MusRequest, config).await;
            return Ok(());
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config: config::Config = {
        if let Ok(mut file) = File::options()
            .read(true)
            .write(false)
            .open("teleconfig.toml")
        {
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .expect("Unable to read config!");
            if let Ok(config) = toml::from_str(&contents) {
                config
            } else {
                panic!("Config malformed!")
            }
        } else {
            panic!("Unable to open config file teleconfig.toml")
        }
    };
    let token = config.telegram_token.clone();

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
                        let config_clone = config.clone();
                        tokio::spawn(async move {
                            if let Err(error) =
                                match_handles(api_clone.clone(), message.clone(), config_clone)
                                    .await
                            {
                                println!("{}", error);
                                let error_params = SendMessageParams::builder()
                                    .chat_id(message.chat.id)
                                    .text(error.to_string())
                                    .build();
                                let _ = api_clone.send_message(&error_params).await;
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
