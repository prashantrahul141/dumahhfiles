use crate::state::CONFIG;

use std::{
    env,
    ffi::OsStr,
    fmt::Debug,
    hash::{DefaultHasher, Hash, Hasher},
    str::FromStr,
    time::Duration,
};

pub fn hash_one<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

pub fn retention_time(file_size: u64) -> Duration {
    // the equation blows if you provide file size bigger than max file size.
    if file_size > CONFIG.max_file_size as u64 {
        return Duration::from_secs(0);
    }

    let mins = CONFIG.min_retention_mns
        + (CONFIG.max_retention_mns
            * (1_f32 - (file_size as f32 / (CONFIG.max_file_size) as f32))
                .powf(std::f32::consts::E));
    Duration::from_secs((mins * 60.0) as u64)
}

pub fn env_or<E, T>(key: E, default: T) -> T
where
    E: AsRef<OsStr>,
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    match env::var(key) {
        Ok(v) => v.parse::<T>().unwrap(),
        Err(_) => default,
    }
}
