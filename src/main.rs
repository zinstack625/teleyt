use db::HasID;
use frankenstein::*;
use log::{info, warn};
use std::fs::File;
use std::io::prelude::*;
use user_status::UserStatus;

mod chat_config;
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
        if let Ok(status) =
            db::get_status(*message.from.as_ref().unwrap().clone(), config.clone()).await
        {
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
        if msg_text.starts_with("/config") {
            let _ = handles::group_config(api, message, config).await;
            return Ok(());
        } else if msg_text.starts_with("/selfconfig") {
            let _ = handles::user_config(api, message, config).await;
            return Ok(());
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
            let _ = handles::set_status(api, *message.chat, UserStatus::VidRequest, config).await;
            return Ok(());
        } else if msg_text.starts_with("/mus") {
            let _ = handles::set_status(api, *message.chat, UserStatus::MusRequest, config).await;
            return Ok(());
        }
    }
    Ok(())
}

async fn match_queries(
    api: AsyncApi,
    query: CallbackQuery,
    config: config::Config,
) -> Result<(), Error> {
    info!("{}", &query.data.as_ref().unwrap()[..3]);
    match &query.data.as_ref().unwrap()[..3] {
        "cfg" => {
            if &query.data.as_ref().unwrap()[4..8] == "user" {
                let _ = chat_config::config_delete(
                    api.clone(),
                    query.clone(),
                    config,
                    match &query.data.unwrap().chars().nth(9).unwrap() {
                        '1' => true,
                        _ => false,
                    },
                )
                .await;
            } else if &query.data.as_ref().unwrap()[4..9] == "group" {
                let _ = chat_config::config_delete(
                    api.clone(),
                    query.clone(),
                    config,
                    match &query.data.unwrap().chars().nth(9).unwrap() {
                        '1' => true,
                        _ => false,
                    },
                )
                .await;
            }
        }
        _ => {
            warn!("unsupported query {}", query.data.unwrap());
        }
    }
    let ok_msg = frankenstein::SendMessageParams::builder()
        .chat_id(query.from.get_id())
        .text("Ok, done")
        .disable_notification(true)
        .build();
    let del_msg_params = frankenstein::DeleteMessageParams::builder()
        .chat_id(query.message.as_ref().unwrap().chat.get_id())
        .message_id(query.message.as_ref().unwrap().message_id)
        .build();
    let answ_cbk_params = frankenstein::AnswerCallbackQueryParams::builder()
        .callback_query_id(query.id)
        .build();
    let _ = tokio::join!(
        api.send_message(&ok_msg),
        api.delete_message(&del_msg_params,),
        api.answer_callback_query(&answ_cbk_params,)
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();
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
    let update_params_builder = GetUpdatesParams::builder()
        .allowed_updates(vec![AllowedUpdate::Message, AllowedUpdate::CallbackQuery]);

    let mut update_params = update_params_builder.clone().build();

    loop {
        let results = api.get_updates(&update_params).await;
        match results {
            Ok(response) => {
                for update in response.result {
                    match update.content {
                        UpdateContent::Message(message) => {
                            let api_clone = api.clone();
                            let config_clone = config.clone();
                            tokio::spawn(async move {
                                if let Err(error) =
                                    match_handles(api_clone.clone(), message.clone(), config_clone)
                                        .await
                                {
                                    warn!("{error}");
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
                        UpdateContent::CallbackQuery(query) => {
                            let api = api.clone();
                            let config = config.clone();
                            tokio::spawn(async move {
                                if let Err(error) =
                                    match_queries(api.clone(), query.clone(), config).await
                                {
                                    warn!("Something went horribly wrong in queries: {error}");
                                    let error_params = SendMessageParams::builder()
                                        .chat_id(query.message.unwrap().chat.get_id())
                                        .text(error.to_string())
                                        .build();
                                    let _ = api.send_message(&error_params).await;
                                }
                            });
                            update_params = update_params_builder
                                .clone()
                                .offset(update.update_id + 1)
                                .build();
                        }
                        _ => {}
                    }
                }
            }
            Err(error) => {
                println!("Failed to get updates! {:?}", error);
            }
        }
    }
}
