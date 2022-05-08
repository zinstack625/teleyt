use redis::Commands;

use crate::user_status::{self, UserStatus};

pub enum DbError {
    Error,
}

pub fn open_redis() -> Option<redis::Connection> {
    let address = std::env::var("REDIS_ADDRESS");
    if let Ok(address) = address {
        let client = redis::Client::open(address);
        match client {
            Ok(client) => {
                if let Ok(con) = client.get_connection() {
                    Some(con)
                } else {
                    None
                }
            }
            Err(err) => {
                println!("{}", err);
                None
            }
        }
    } else {
        None
    }
}

pub async fn set_user_status(
    user: frankenstein::Chat,
    status: user_status::UserStatus,
) -> Result<(), DbError> {
    let mut con = open_redis();
    if con.is_some() {
        let con = con.as_mut().unwrap();
        let mut key = user.id.to_string();
        key.push_str(":status");
        if let Err(error) = redis::pipe()
            .set(&key, status as i32)
            .ignore()
            .expire(&key, 60)
            .ignore()
            .query::<()>(con)
        {
            println!("{}", error);
            return Err(DbError::Error);
        }
        Ok(())
    } else {
        Err(DbError::Error)
    }
}

pub async fn get_user_status(
    user: frankenstein::Chat,
) -> Result<crate::user_status::UserStatus, DbError> {
    let mut con = open_redis();
    if con.is_some() {
        let con = con.as_mut().unwrap();
        let mut key = user.id.to_string();
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
