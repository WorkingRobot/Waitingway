use actix_web::{
    error::ErrorUnauthorized, http::header::USER_AGENT, Error, FromRequest, HttpRequest,
};

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserAgentVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub configuration: String,
}

impl UserAgentVersion {
    // Waitingway/0.0.0-Debug or Waitingway/0.0.0-Release
    pub fn from_string(value: &str) -> Option<Self> {
        let mut parts = value.split('/');

        let name = parts.next()?;
        if name != "Waitingway" {
            return None;
        }

        let version_cfg = parts.next()?;
        let mut version_cfg = version_cfg.split('-');

        let version = version_cfg.next()?;

        let mut version = version.split('.');

        let major = version.next()?.parse().ok()?;
        let minor = version.next()?.parse().ok()?;
        let patch = version.next()?.parse().ok()?;

        let configuration = version_cfg.next()?.to_owned();

        Some(Self {
            major,
            minor,
            patch,
            configuration,
        })
    }
}

impl std::fmt::Display for UserAgentVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{major}.{minor}.{patch}-{configuration}",
            major = self.major,
            minor = self.minor,
            patch = self.patch,
            configuration = self.configuration
        )
    }
}

impl TryFrom<&str> for UserAgentVersion {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        UserAgentVersion::from_string(value).ok_or(())
    }
}

impl FromRequest for UserAgentVersion {
    type Error = Error;
    type Future = futures_util::future::Ready<Result<Self, Error>>;

    fn from_request(req: &HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let header = req
            .headers()
            .get(USER_AGENT)
            .ok_or(ErrorUnauthorized("No User-Agent found"));
        let header_str = header.and_then(|v| {
            v.to_str()
                .ok()
                .ok_or(ErrorUnauthorized("Invalid User-Agent"))
        });
        let version = header_str.and_then(|v| {
            UserAgentVersion::from_string(v).ok_or(ErrorUnauthorized("Invalid User-Agent version"))
        });
        futures_util::future::ready(version)
    }
}
