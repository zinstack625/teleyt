use redis::{FromRedisValue, RedisResult, Value};

pub enum UserStatus {
    VidRequest = 1,
    MusRequest,
    None,
}

impl FromRedisValue for UserStatus {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        match v {
            Value::Int(status) => match status {
                1 => Ok(UserStatus::VidRequest),
                2 => Ok(UserStatus::MusRequest),
                _ => Ok(UserStatus::None),
            },
            _ => RedisResult::Err(redis::RedisError::from((
                redis::ErrorKind::ExtensionError,
                "Invalid type in database",
            ))),
        }
    }

    fn from_redis_values(items: &[Value]) -> RedisResult<Vec<Self>> {
        panic!("Not implemented for many values!");
    }
}
