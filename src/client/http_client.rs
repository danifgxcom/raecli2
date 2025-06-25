use crate::config::Config;
use crate::types::{RaeError, Result};
use crate::utils::{ClientType, HeaderBuilder};

pub struct HttpClient {
    config: Config,
}

pub struct HttpResponse {
    pub content: String,
    pub status_code: u16,
    pub is_challenge: bool,
}

impl HttpClient {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn fetch_page(&self, word: &str) -> Result<HttpResponse> {
        let url = format!("https://dle.rae.es/{}", urlencoding::encode(word));

        // Primary method: Custom TLS client
        match self.fetch_with_tls_client(&url) {
            Ok(response) => return Ok(response),
            Err(e) => {
                if self.config.debug.enabled {
                    eprintln!("TLS client failed: {}, trying fallback", e);
                }
            }
        }

        // Fallback: Standard HTTP client
        self.fetch_with_reqwest(&url, word)
    }

    fn fetch_with_tls_client(&self, _url: &str) -> Result<HttpResponse> {
        Err(RaeError::ConfigError("TLS client not implemented".to_string()))
    }

    fn fetch_with_reqwest(&self, url: &str, _word: &str) -> Result<HttpResponse> {
        let client = self.create_reqwest_client()?;
        let headers =
            HeaderBuilder::new(ClientType::Lynx, self.config.user_agents.clone()).build()?;

        let response = client
            .get(url)
            .headers(headers)
            .send()
            .map_err(RaeError::from)?;

        let status_code = response.status().as_u16();
        let content = response.text().map_err(RaeError::from)?;
        let is_challenge = self.detect_cloudflare_challenge(&content);

        Ok(HttpResponse {
            content,
            status_code,
            is_challenge,
        })
    }

    fn create_reqwest_client(&self) -> Result<reqwest::blocking::Client> {
        reqwest::blocking::Client::builder()
            .user_agent(&self.config.user_agents.lynx)
            .cookie_store(true)
            .timeout(self.config.timeouts.request)
            .connect_timeout(self.config.timeouts.connect)
            .redirect(reqwest::redirect::Policy::limited(3))
            .tcp_nodelay(false)
            .tcp_keepalive(Some(std::time::Duration::from_secs(120)))
            .pool_idle_timeout(Some(std::time::Duration::from_secs(60)))
            .pool_max_idle_per_host(1)
            .http1_only()
            .no_brotli()
            .build()
            .map_err(RaeError::from)
    }

    fn detect_cloudflare_challenge(&self, html: &str) -> bool {
        // Quick return for successful responses
        if html.starts_with("HTTP/1.1 200 OK") {
            return false;
        }

        // Check for RAE content
        if (html.contains("<title>")
            && html.contains("Definición | Diccionario de la lengua española | RAE"))
            || (html.contains("Diccionario de la lengua española")
                && html.contains("Real Academia Española"))
        {
            return false;
        }

        // Detect challenge indicators
        let challenge_indicators = [
            "Checking your browser",
            "Please wait",
            "Just a moment",
            "__cf_chl_jschl_tk__",
            "DDoS protection",
            "challenge-form",
            "challenge-error-title",
            "cf_chl_prog",
            "cf-please-wait",
            "cf-browser-verification",
            "turnstile",
            "cf_challenge",
        ];

        for indicator in &challenge_indicators {
            if html.contains(indicator) {
                return true;
            }
        }

        // Check for 403 Forbidden
        if html.contains("403 Forbidden") && html.contains("cloudflare") {
            return true;
        }

        // Small response without useful content
        if html.len() < 1000
            && !html.contains("<article")
            && !html.contains("<body")
            && !html.contains("HTTP/1.1")
        {
            return true;
        }

        false
    }
}
