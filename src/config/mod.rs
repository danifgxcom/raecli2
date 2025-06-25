use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub timeouts: TimeoutConfig,
    pub user_agents: UserAgentConfig,
    pub selectors: SelectorConfig,
    pub exclusions: Vec<String>,
    pub debug: DebugConfig,
}

#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub connect: Duration,
    pub request: Duration,
    pub challenge_wait: Duration,
    pub lynx_timing_before_connect: Duration,
    pub lynx_timing_before_handshake: Duration,
}

#[derive(Debug, Clone)]
pub struct UserAgentConfig {
    pub lynx: String,
    pub chrome: String,
    pub firefox: String,
    pub curl: String,
}

#[derive(Debug, Clone)]
pub struct SelectorConfig {
    pub definition_selectors: Vec<String>,
    pub abbreviation_selector: String,
    pub number_selector: String,
}

#[derive(Debug, Clone)]
pub struct DebugConfig {
    pub enabled: bool,
    pub save_html: bool,
    pub save_logs: bool,
    pub verbose_tls: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timeouts: TimeoutConfig::default(),
            user_agents: UserAgentConfig::default(),
            selectors: SelectorConfig::default(),
            exclusions: default_exclusions(),
            debug: DebugConfig::default(),
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(30),
            request: Duration::from_secs(60),
            challenge_wait: Duration::from_secs(5),
            lynx_timing_before_connect: Duration::from_millis(150),
            lynx_timing_before_handshake: Duration::from_millis(200),
        }
    }
}

impl Default for UserAgentConfig {
    fn default() -> Self {
        Self {
            lynx: "Lynx/2.9.0 libwww-FM/2.14 SSL-MM/1.4.1 GNUTLS/3.8.3".to_string(),
            chrome: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36".to_string(),
            firefox: "Mozilla/5.0 (X11; Linux x86_64; rv:132.0) Gecko/20100101 Firefox/132.0".to_string(),
            curl: "curl/8.5.0".to_string(),
        }
    }
}

impl Default for SelectorConfig {
    fn default() -> Self {
        Self {
            definition_selectors: vec![
                ".c-definitions__item[role='definition']".to_string(),
                ".c-definitions li".to_string(),
                "article p".to_string(),
                "article ol li".to_string(),
                "article ul li".to_string(),
                ".normal p".to_string(),
                ".normal li".to_string(),
                "#resultados p".to_string(),
                "#resultados li".to_string(),
            ],
            abbreviation_selector: "abbr.d".to_string(),
            number_selector: ".n_acep".to_string(),
        }
    }
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            save_html: true,
            save_logs: true,
            verbose_tls: false,
        }
    }
}

fn default_exclusions() -> Vec<String> {
    vec![
        "acceso",
        "nueva búsqueda",
        "búsqueda avanzada",
        "consultar",
        "real academia española",
        "diccionario",
        "versión electrónica",
        "fundación",
        "nueva búsqueda",
        "avanzada",
        "conjugar",
        "cerrar",
        "compartir",
        "imprimir",
        "sinónimos",
        "antónimos",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}
