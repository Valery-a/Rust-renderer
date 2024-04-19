use redis::Commands;
use std::env;

pub fn cache_user(user: &User) -> redis::RedisResult<()> {
    let client = redis::Client::open(env::var("REDIS_URL").expect("REDIS_URL must be set"))?;
    let mut con = client.get_connection()?;
    let _: () = con.set_ex(format!("user:{}", user.id), serde_json::to_string(user)?, 3600)?;  // Cache for 1 hour
    Ok(())
}

pub fn get_cached_user(user_id: i32) -> redis::RedisResult<Option<User>> {
    let client = redis::Client::open(env::var("REDIS_URL").expect("REDIS_URL must be set"))?;
    let mut con = client.get_connection()?;
    let user_json: Option<String> = con.get(format!("user:{}", user_id))?;
    Ok(user_json.map(|json| serde_json::from_str(&json).unwrap()))
}
