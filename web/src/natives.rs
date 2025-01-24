use konst::{
    option,
    primitive::{parse_i64, parse_u32},
    result,
};
use serde::Serialize;
use std::time::SystemTime;
use time::{Duration, OffsetDateTime};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Platform error")]
    PlatformError(#[from] anyhow::Error),
}

type Result<T> = std::result::Result<T, Error>;

#[cfg(target_os = "windows")]
fn process_start_time_imp() -> Result<SystemTime> {
    use nt_time::FileTime;
    use windows::Win32::{
        Foundation::FILETIME,
        System::Threading::{GetCurrentProcess, GetProcessTimes},
    };

    unsafe {
        let mut creation_time = FILETIME::default();
        GetProcessTimes(
            GetCurrentProcess(),
            &mut creation_time,
            &mut FILETIME::default(),
            &mut FILETIME::default(),
            &mut FILETIME::default(),
        )
        .map_err(anyhow::Error::from)?;
        Ok(
            FileTime::from_high_low(creation_time.dwHighDateTime, creation_time.dwLowDateTime)
                .into(),
        )
    }
}

#[cfg(target_os = "linux")]
fn process_start_time_imp() -> Result<SystemTime> {
    use std::time::Duration;

    use procfs::{boot_time_secs, process::Process, ticks_per_second};

    Process::myself().and_then(|p| p.stat()).and_then(|stat| {
        let seconds_since_boot = stat.starttime as f64 / ticks_per_second() as f64;

        Ok(SystemTime::UNIX_EPOCH
            + Duration::from_secs(boot_time_secs()?)
            + Duration::from_secs_f64(seconds_since_boot))
    })
}

pub fn process_start_time() -> Result<OffsetDateTime> {
    Ok(process_start_time_imp()?.into())
}

pub fn process_uptime() -> Result<Duration> {
    Ok(OffsetDateTime::now_utc() - process_start_time()?)
}

#[derive(Debug, Serialize)]
pub struct VersionData {
    pub name: &'static str,
    pub authors: &'static str,
    pub description: &'static str,
    pub repository: &'static str,
    pub profile: &'static str,
    pub version: &'static str,
    pub version_major: u32,
    pub version_minor: u32,
    pub version_patch: u32,
    #[serde(with = "time::serde::rfc3339")]
    pub build_time: time::OffsetDateTime,
}

pub const VERSION_DATA: VersionData = VersionData {
    name: env!("CARGO_PKG_NAME"),
    authors: env!("CARGO_PKG_AUTHORS"),
    description: env!("CARGO_PKG_DESCRIPTION"),
    repository: env!("CARGO_PKG_REPOSITORY"),
    profile: env!("PROFILE"),
    version: env!("CARGO_PKG_VERSION"),
    version_major: result::unwrap_ctx!(parse_u32(env!("CARGO_PKG_VERSION_MAJOR"))),
    version_minor: result::unwrap_ctx!(parse_u32(env!("CARGO_PKG_VERSION_MINOR"))),
    version_patch: result::unwrap_ctx!(parse_u32(env!("CARGO_PKG_VERSION_PATCH"))),
    build_time: option::unwrap!(result::ok!(time::OffsetDateTime::from_unix_timestamp(
        result::unwrap_ctx!(parse_i64(env!("BUILD_TIMESTAMP")))
    ))),
};
