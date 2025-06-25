use crate::config::UserAgentConfig;
use crate::types::Result;
use reqwest::header::{HeaderMap, HeaderValue};

#[derive(Debug, Clone)]
pub enum ClientType {
    Lynx,
    Chrome,
    Firefox,
    Curl,
}

pub struct HeaderBuilder {
    client_type: ClientType,
    user_agents: UserAgentConfig,
}

impl HeaderBuilder {
    pub fn new(client_type: ClientType, user_agents: UserAgentConfig) -> Self {
        Self {
            client_type,
            user_agents,
        }
    }

    pub fn build(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();

        match self.client_type {
            ClientType::Lynx => self.build_lynx_headers(&mut headers)?,
            ClientType::Chrome => self.build_chrome_headers(&mut headers)?,
            ClientType::Firefox => self.build_firefox_headers(&mut headers)?,
            ClientType::Curl => self.build_curl_headers(&mut headers)?,
        }

        Ok(headers)
    }

    fn build_lynx_headers(&self, headers: &mut HeaderMap) -> Result<()> {
        headers.insert("Accept", HeaderValue::from_static("text/html"));
        headers.insert("Accept-Language", HeaderValue::from_static("es"));
        headers.insert("Accept-Encoding", HeaderValue::from_static("gzip"));
        headers.insert(
            "User-Agent",
            HeaderValue::from_str(&self.user_agents.lynx).map_err(|e| {
                crate::types::RaeError::ConfigError(format!("Invalid user agent: {}", e))
            })?,
        );
        Ok(())
    }

    fn build_chrome_headers(&self, headers: &mut HeaderMap) -> Result<()> {
        headers.insert("Accept", HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"
        ));
        headers.insert("Accept-Language", HeaderValue::from_static("en,es;q=0.9"));
        headers.insert(
            "Accept-Encoding",
            HeaderValue::from_static("gzip, deflate, br, zstd"),
        );
        headers.insert("Cache-Control", HeaderValue::from_static("max-age=0"));
        headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("document"));
        headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("navigate"));
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));
        headers.insert("Sec-Fetch-User", HeaderValue::from_static("?1"));
        headers.insert(
            "Sec-Ch-Ua",
            HeaderValue::from_static(
                "\"Chromium\";v=\"136\", \"Google Chrome\";v=\"136\", \"Not.A/Brand\";v=\"99\"",
            ),
        );
        headers.insert("Sec-Ch-Ua-Mobile", HeaderValue::from_static("?0"));
        headers.insert("Sec-Ch-Ua-Platform", HeaderValue::from_static("\"Linux\""));
        headers.insert("Upgrade-Insecure-Requests", HeaderValue::from_static("1"));
        headers.insert("Priority", HeaderValue::from_static("u=0, i"));
        headers.insert(
            "User-Agent",
            HeaderValue::from_str(&self.user_agents.chrome).map_err(|e| {
                crate::types::RaeError::ConfigError(format!("Invalid user agent: {}", e))
            })?,
        );
        Ok(())
    }

    fn build_firefox_headers(&self, headers: &mut HeaderMap) -> Result<()> {
        headers.insert("Accept", HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"
        ));
        headers.insert(
            "Accept-Language",
            HeaderValue::from_static("es-ES,es;q=0.8,en-US;q=0.5,en;q=0.3"),
        );
        headers.insert(
            "Accept-Encoding",
            HeaderValue::from_static("gzip, deflate, br"),
        );
        headers.insert("Upgrade-Insecure-Requests", HeaderValue::from_static("1"));
        headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("document"));
        headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("navigate"));
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));
        headers.insert(
            "User-Agent",
            HeaderValue::from_str(&self.user_agents.firefox).map_err(|e| {
                crate::types::RaeError::ConfigError(format!("Invalid user agent: {}", e))
            })?,
        );
        Ok(())
    }

    fn build_curl_headers(&self, headers: &mut HeaderMap) -> Result<()> {
        headers.insert("Accept", HeaderValue::from_static("*/*"));
        headers.insert(
            "User-Agent",
            HeaderValue::from_str(&self.user_agents.curl).map_err(|e| {
                crate::types::RaeError::ConfigError(format!("Invalid user agent: {}", e))
            })?,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_user_agents() -> UserAgentConfig {
        UserAgentConfig::default()
    }

    #[test]
    fn test_lynx_headers() {
        let user_agents = create_test_user_agents();
        let builder = HeaderBuilder::new(ClientType::Lynx, user_agents);
        let headers = builder.build().unwrap();

        // Check required headers
        assert!(headers.contains_key("Accept"));
        assert!(headers.contains_key("Accept-Language"));
        assert!(headers.contains_key("Accept-Encoding"));
        assert!(headers.contains_key("User-Agent"));

        // Check specific values
        assert_eq!(headers.get("Accept").unwrap(), "text/html");
        assert_eq!(headers.get("Accept-Language").unwrap(), "es");
        assert_eq!(headers.get("Accept-Encoding").unwrap(), "gzip");
        assert!(headers
            .get("User-Agent")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("Lynx"));
    }

    #[test]
    fn test_chrome_headers() {
        let user_agents = create_test_user_agents();
        let builder = HeaderBuilder::new(ClientType::Chrome, user_agents);
        let headers = builder.build().unwrap();

        // Check Chrome-specific headers
        assert!(headers.contains_key("Sec-Ch-Ua"));
        assert!(headers.contains_key("Sec-Ch-Ua-Mobile"));
        assert!(headers.contains_key("Sec-Ch-Ua-Platform"));
        assert!(headers.contains_key("Sec-Fetch-Dest"));
        assert!(headers.contains_key("Sec-Fetch-Mode"));
        assert!(headers.contains_key("Sec-Fetch-Site"));
        assert!(headers.contains_key("Sec-Fetch-User"));
        assert!(headers.contains_key("Priority"));

        // Check values
        assert_eq!(headers.get("Sec-Fetch-Dest").unwrap(), "document");
        assert_eq!(headers.get("Sec-Fetch-Mode").unwrap(), "navigate");
        assert_eq!(headers.get("Sec-Ch-Ua-Mobile").unwrap(), "?0");
        assert!(headers
            .get("User-Agent")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("Chrome"));
    }

    #[test]
    fn test_firefox_headers() {
        let user_agents = create_test_user_agents();
        let builder = HeaderBuilder::new(ClientType::Firefox, user_agents);
        let headers = builder.build().unwrap();

        // Check Firefox-specific characteristics
        assert!(headers.contains_key("Accept"));
        assert!(headers.contains_key("Accept-Language"));
        assert!(headers.contains_key("Accept-Encoding"));
        assert!(headers.contains_key("Sec-Fetch-Dest"));

        // Firefox should not have Chrome-specific headers
        assert!(!headers.contains_key("Sec-Ch-Ua"));
        assert!(!headers.contains_key("Priority"));

        // Check Firefox user agent
        assert!(headers
            .get("User-Agent")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("Firefox"));
    }

    #[test]
    fn test_curl_headers() {
        let user_agents = create_test_user_agents();
        let builder = HeaderBuilder::new(ClientType::Curl, user_agents);
        let headers = builder.build().unwrap();

        // Curl should have minimal headers
        assert!(headers.contains_key("Accept"));
        assert!(headers.contains_key("User-Agent"));

        // Should not have browser-specific headers
        assert!(!headers.contains_key("Sec-Fetch-Dest"));
        assert!(!headers.contains_key("Sec-Ch-Ua"));

        // Check values
        assert_eq!(headers.get("Accept").unwrap(), "*/*");
        assert!(headers
            .get("User-Agent")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("curl"));
    }

    #[test]
    fn test_header_builder_different_client_types() {
        let user_agents = create_test_user_agents();

        let lynx_headers = HeaderBuilder::new(ClientType::Lynx, user_agents.clone())
            .build()
            .unwrap();
        let chrome_headers = HeaderBuilder::new(ClientType::Chrome, user_agents.clone())
            .build()
            .unwrap();
        let firefox_headers = HeaderBuilder::new(ClientType::Firefox, user_agents.clone())
            .build()
            .unwrap();
        let curl_headers = HeaderBuilder::new(ClientType::Curl, user_agents)
            .build()
            .unwrap();

        // Each client type should produce different headers
        assert_ne!(lynx_headers.len(), chrome_headers.len());
        assert_ne!(firefox_headers.len(), curl_headers.len());

        // User agents should be different
        assert_ne!(
            lynx_headers.get("User-Agent").unwrap(),
            chrome_headers.get("User-Agent").unwrap()
        );
    }

    #[test]
    fn test_lynx_headers_are_minimal() {
        let user_agents = create_test_user_agents();
        let headers = HeaderBuilder::new(ClientType::Lynx, user_agents)
            .build()
            .unwrap();

        // Lynx should have the fewest headers (just the essentials)
        assert_eq!(headers.len(), 4); // Accept, Accept-Language, Accept-Encoding, User-Agent

        // Should not have modern browser headers
        assert!(!headers.contains_key("Sec-Fetch-Dest"));
        assert!(!headers.contains_key("Sec-Ch-Ua"));
        assert!(!headers.contains_key("Priority"));
    }

    #[test]
    fn test_chrome_headers_are_comprehensive() {
        let user_agents = create_test_user_agents();
        let headers = HeaderBuilder::new(ClientType::Chrome, user_agents)
            .build()
            .unwrap();

        // Chrome should have many headers (modern browser)
        assert!(headers.len() >= 10);

        // Should have all the modern browser headers
        let required_chrome_headers = [
            "Accept",
            "Accept-Language",
            "Accept-Encoding",
            "Cache-Control",
            "Sec-Fetch-Dest",
            "Sec-Fetch-Mode",
            "Sec-Fetch-Site",
            "Sec-Fetch-User",
            "Sec-Ch-Ua",
            "Sec-Ch-Ua-Mobile",
            "Sec-Ch-Ua-Platform",
            "Upgrade-Insecure-Requests",
            "Priority",
            "User-Agent",
        ];

        for header in &required_chrome_headers {
            assert!(headers.contains_key(*header), "Missing header: {}", header);
        }
    }

    #[test]
    fn test_invalid_user_agent_handling() {
        let mut user_agents = create_test_user_agents();
        // Insert an invalid header value (contains newline)
        user_agents.lynx = "Invalid\nUser\rAgent".to_string();

        let builder = HeaderBuilder::new(ClientType::Lynx, user_agents);
        let result = builder.build();

        // Should return an error for invalid header values
        assert!(result.is_err());
    }

    #[test]
    fn test_header_values_are_valid() {
        let user_agents = create_test_user_agents();
        let client_types = [
            ClientType::Lynx,
            ClientType::Chrome,
            ClientType::Firefox,
            ClientType::Curl,
        ];

        for client_type in &client_types {
            let headers = HeaderBuilder::new(client_type.clone(), user_agents.clone())
                .build()
                .unwrap();

            // All header values should be valid HTTP header values
            for (name, value) in headers.iter() {
                assert!(!name.as_str().is_empty(), "Header name should not be empty");
                assert!(
                    value.to_str().is_ok(),
                    "Header value should be valid ASCII: {:?}",
                    value
                );
            }
        }
    }

    #[test]
    fn test_header_builder_consistency() {
        let user_agents = create_test_user_agents();
        let builder = HeaderBuilder::new(ClientType::Lynx, user_agents.clone());

        // Building headers multiple times should produce the same result
        let headers1 = builder.build().unwrap();
        let builder2 = HeaderBuilder::new(ClientType::Lynx, user_agents);
        let headers2 = builder2.build().unwrap();

        assert_eq!(headers1.len(), headers2.len());

        for (name, value) in headers1.iter() {
            assert_eq!(headers2.get(name), Some(value));
        }
    }

    #[test]
    fn test_accept_encoding_differences() {
        let user_agents = create_test_user_agents();

        let lynx_headers = HeaderBuilder::new(ClientType::Lynx, user_agents.clone())
            .build()
            .unwrap();
        let chrome_headers = HeaderBuilder::new(ClientType::Chrome, user_agents)
            .build()
            .unwrap();

        // Lynx should have simpler Accept-Encoding
        assert_eq!(lynx_headers.get("Accept-Encoding").unwrap(), "gzip");

        // Chrome should support more compression formats
        let chrome_encoding = chrome_headers
            .get("Accept-Encoding")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(chrome_encoding.contains("gzip"));
        assert!(chrome_encoding.contains("deflate"));
        assert!(chrome_encoding.contains("br"));
    }
}
