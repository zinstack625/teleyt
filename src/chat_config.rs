use frankenstein::{AsyncApi, AsyncTelegramApi, CallbackQuery};
use log::info;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use crate::{
    config,
    db::{self, HasID},
};

#[derive(Serialize, Deserialize, TypedBuilder)]
#[builder(field_defaults(default, setter(strip_option)))]
pub struct ChatConfig {
    #[builder(default = Some(false))]
    pub delete_on_send: Option<bool>,
}

pub async fn config_delete(
    api: AsyncApi,
    query: CallbackQuery,
    config: config::Config,
    delete: bool,
) {
    info!("setting {} config to {}", query.from.get_id(), &delete);
    let _ = db::set_config(
        query.from.clone(),
        ChatConfig::builder().delete_on_send(delete).build(),
        config.clone(),
    )
    .await;
}
