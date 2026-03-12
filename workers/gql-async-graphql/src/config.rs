use worker::Env;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeConfig {
    supabase_url: String,
    supabase_publishable_key: String,
}

impl RuntimeConfig {
    pub fn new(
        supabase_url: impl Into<String>,
        supabase_publishable_key: impl Into<String>,
    ) -> Self {
        Self {
            supabase_url: normalize_base_url(supabase_url.into()),
            supabase_publishable_key: supabase_publishable_key.into(),
        }
    }

    pub fn from_env(env: &Env) -> Result<Self, ConfigError> {
        let supabase_url = env
            .var("SUPABASE_URL")
            .map_err(|_| ConfigError::MissingVar("SUPABASE_URL"))?
            .to_string();
        let supabase_publishable_key = env
            .var("SUPABASE_PUBLISHABLE_KEY")
            .map_err(|_| ConfigError::MissingVar("SUPABASE_PUBLISHABLE_KEY"))?
            .to_string();

        if supabase_url.trim().is_empty() {
            return Err(ConfigError::InvalidVar("SUPABASE_URL"));
        }
        if supabase_publishable_key.trim().is_empty() {
            return Err(ConfigError::InvalidVar("SUPABASE_PUBLISHABLE_KEY"));
        }

        Ok(Self::new(supabase_url, supabase_publishable_key))
    }

    pub fn supabase_publishable_key(&self) -> &str {
        &self.supabase_publishable_key
    }

    pub fn flights_rest_url(&self) -> String {
        format!("{}/rest/v1/flights", self.supabase_url)
    }

    pub fn jwt_issuer(&self) -> String {
        format!("{}/auth/v1", self.supabase_url)
    }

    pub fn jwks_url(&self) -> String {
        format!("{}/auth/v1/.well-known/jwks.json", self.supabase_url)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    MissingVar(&'static str),
    InvalidVar(&'static str),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingVar(name) => write!(f, "Missing required variable {name}"),
            ConfigError::InvalidVar(name) => write!(f, "Invalid required variable {name}"),
        }
    }
}

fn normalize_base_url(url: String) -> String {
    url.trim().trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_supabase_urls_without_double_slashes() {
        let config = RuntimeConfig::new(
            "https://example.supabase.co/",
            "publishable-key",
        );

        assert_eq!(
            config.flights_rest_url(),
            "https://example.supabase.co/rest/v1/flights",
        );
        assert_eq!(
            config.jwt_issuer(),
            "https://example.supabase.co/auth/v1",
        );
        assert_eq!(
            config.jwks_url(),
            "https://example.supabase.co/auth/v1/.well-known/jwks.json",
        );
    }
}
