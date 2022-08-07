use redis::Commands;

use crate::chat_config;
use crate::user_status::UserStatus;
use log::{debug, error, info};
//use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum DbError {
    Error,
}

pub trait HasID {
    fn get_id(&self) -> i64;
}

impl HasID for frankenstein::User {
    fn get_id(&self) -> i64 {
        self.id as i64
    }
}

impl HasID for frankenstein::Chat {
    fn get_id(&self) -> i64 {
        self.id
    }
}

pub fn open_redis(config: crate::config::Config) -> Option<redis::Connection> {
    if config.redis_address.is_none() {
        return None;
    }
    match redis::Client::open(&config.redis_address.as_ref().unwrap()[..]) {
        Ok(client) => {
            if let Ok(con) = client.get_connection() {
                Some(con)
            } else {
                None
            }
        }
        Err(err) => {
            error!("{}", err);
            None
        }
    }
}

pub async fn set_status<T: HasID>(
    ent: T,
    status: UserStatus,
    config: crate::config::Config,
) -> Result<(), DbError> {
    let mut con = open_redis(config);
    if con.is_some() {
        let con = con.as_mut().unwrap();
        let mut key = ent.get_id().to_string();
        key.push_str(":status");
        if let Err(error) = redis::pipe()
            .set(&key, status as i32)
            .ignore()
            .expire(&key, 60)
            .ignore()
            .query::<()>(con)
        {
            error!("{}", error);
            return Err(DbError::Error);
        }
        Ok(())
    } else {
        Err(DbError::Error)
    }
}

pub async fn set_config<T: HasID>(
    ent: T,
    chat_conf: chat_config::ChatConfig,
    config: crate::config::Config,
) -> Result<(), DbError> {
    let mut con = open_redis(config);
    if con.is_none() {
        return Err(DbError::Error);
    }
    let con = con.as_mut().unwrap();
    let mut key = ent.get_id().to_string();
    key.push_str(":config");
    if let Err(error) = redis::pipe()
        .set(&key, serde_json::to_string(&chat_conf).unwrap())
        .ignore()
        .query::<()>(con)
    {
        error!("{}", error);
        return Err(DbError::Error);
    }
    Ok(())
}

pub async fn get_status<T: HasID>(
    ent: T,
    config: crate::config::Config,
) -> Result<UserStatus, DbError> {
    let mut con = open_redis(config);
    if con.is_some() {
        let con = con.as_mut().unwrap();
        let mut key = ent.get_id().to_string();
        key.push_str(":status");
        let status = con.get(&key).unwrap_or(0i32);
        match status {
            1 => Ok(UserStatus::VidRequest),
            2 => Ok(UserStatus::MusRequest),
            _ => Ok(UserStatus::None),
        }
    } else {
        Err(DbError::Error)
    }
}

pub async fn get_config<T: HasID>(
    ent: T,
    config: crate::config::Config,
) -> Result<chat_config::ChatConfig, DbError> {
    let mut con = open_redis(config);
    if con.is_none() {
        return Err(DbError::Error);
    }
    let con = con.as_mut().unwrap();
    let mut key = ent.get_id().to_string();
    key.push_str(":config");
    match con.get::<&str, Option<String>>(&key) {
        Ok(user_conf_ser) => Ok(serde_json::from_str::<chat_config::ChatConfig>(
            &user_conf_ser.unwrap_or("{'delete_on_send': false}".to_string()),
        )
        .unwrap_or(
            chat_config::ChatConfig::builder()
                .delete_on_send(false)
                .build(),
        )),
        _ => Err(DbError::Error),
    }
}
