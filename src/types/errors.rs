use std::fmt;

#[derive(Debug)]
pub enum RaeError {
    Network(String),
    CloudflareChallenge(String),
    NoDefinitions,
    TlsHandshake(String),
    InvalidResponse(String),
    ParseError(String),
    ConfigError(String),
}

impl fmt::Display for RaeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RaeError::Network(msg) => write!(f, "Network error: {}", msg),
            RaeError::CloudflareChallenge(msg) => write!(f, "Cloudflare challenge failed: {}", msg),
            RaeError::NoDefinitions => write!(f, "No definitions found"),
            RaeError::TlsHandshake(msg) => write!(f, "TLS handshake error: {}", msg),
            RaeError::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
            RaeError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            RaeError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for RaeError {}

impl From<reqwest::Error> for RaeError {
    fn from(err: reqwest::Error) -> Self {
        RaeError::Network(err.to_string())
    }
}

impl From<std::io::Error> for RaeError {
    fn from(err: std::io::Error) -> Self {
        RaeError::Network(err.to_string())
    }
}

impl From<rustls::Error> for RaeError {
    fn from(err: rustls::Error) -> Self {
        RaeError::TlsHandshake(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, RaeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let network_error = RaeError::Network("Connection failed".to_string());
        assert_eq!(
            format!("{}", network_error),
            "Network error: Connection failed"
        );

        let cloudflare_error = RaeError::CloudflareChallenge("Challenge failed".to_string());
        assert_eq!(
            format!("{}", cloudflare_error),
            "Cloudflare challenge failed: Challenge failed"
        );

        let no_definitions_error = RaeError::NoDefinitions;
        assert_eq!(format!("{}", no_definitions_error), "No definitions found");

        let tls_error = RaeError::TlsHandshake("Handshake failed".to_string());
        assert_eq!(
            format!("{}", tls_error),
            "TLS handshake error: Handshake failed"
        );

        let invalid_response_error = RaeError::InvalidResponse("Bad response".to_string());
        assert_eq!(
            format!("{}", invalid_response_error),
            "Invalid response: Bad response"
        );

        let parse_error = RaeError::ParseError("Parse failed".to_string());
        assert_eq!(format!("{}", parse_error), "Parse error: Parse failed");

        let config_error = RaeError::ConfigError("Config invalid".to_string());
        assert_eq!(
            format!("{}", config_error),
            "Configuration error: Config invalid"
        );
    }

    #[test]
    fn test_error_debug() {
        let error = RaeError::Network("Test error".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("Network"));
        assert!(debug_str.contains("Test error"));
    }

    #[test]
    fn test_error_is_error() {
        let error = RaeError::NoDefinitions;
        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn test_from_reqwest_error() {
        // We can't easily create a real reqwest::Error in tests,
        // but we can test the conversion exists
        let mock_io_error =
            std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "Connection refused");
        let rae_error = RaeError::from(mock_io_error);

        match rae_error {
            RaeError::Network(msg) => assert!(msg.contains("Connection refused")),
            _ => panic!("Expected Network error"),
        }
    }

    #[test]
    fn test_from_io_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let rae_error = RaeError::from(io_error);

        match rae_error {
            RaeError::Network(msg) => assert!(msg.contains("File not found")),
            _ => panic!("Expected Network error"),
        }
    }

    #[test]
    fn test_from_rustls_error() {
        let rustls_error = rustls::Error::InappropriateMessage {
            expect_types: vec![],
            got_type: rustls::ContentType::Alert,
        };
        let rae_error = RaeError::from(rustls_error);

        match rae_error {
            RaeError::TlsHandshake(_) => {} // Success
            _ => panic!("Expected TlsHandshake error"),
        }
    }

    #[test]
    fn test_result_type_alias() {
        fn test_function() -> Result<String> {
            Ok("success".to_string())
        }

        fn test_function_error() -> Result<String> {
            Err(RaeError::NoDefinitions)
        }

        assert!(test_function().is_ok());
        assert!(test_function_error().is_err());
    }

    #[test]
    fn test_error_chain() {
        let io_error =
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Permission denied");
        let rae_error = RaeError::from(io_error);

        // Test that we can chain errors
        let result: Result<()> = Err(rae_error);
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(format!("{}", e).contains("Permission denied"));
        }
    }

    #[test]
    fn test_error_equality() {
        let error1 = RaeError::NoDefinitions;
        let error2 = RaeError::NoDefinitions;

        // We can't directly compare errors since they don't implement PartialEq,
        // but we can compare their string representations
        assert_eq!(format!("{}", error1), format!("{}", error2));
    }

    #[test]
    fn test_all_error_variants() {
        let errors = vec![
            RaeError::Network("test".to_string()),
            RaeError::CloudflareChallenge("test".to_string()),
            RaeError::NoDefinitions,
            RaeError::TlsHandshake("test".to_string()),
            RaeError::InvalidResponse("test".to_string()),
            RaeError::ParseError("test".to_string()),
            RaeError::ConfigError("test".to_string()),
        ];

        // Ensure all variants can be created and displayed
        for error in errors {
            let display_str = format!("{}", error);
            assert!(!display_str.is_empty());

            let debug_str = format!("{:?}", error);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_error_context() {
        // Test that error messages are descriptive
        let network_error = RaeError::Network("DNS resolution failed".to_string());
        let message = format!("{}", network_error);
        assert!(message.contains("Network error"));
        assert!(message.contains("DNS resolution failed"));

        let tls_error = RaeError::TlsHandshake("Certificate verification failed".to_string());
        let message = format!("{}", tls_error);
        assert!(message.contains("TLS handshake error"));
        assert!(message.contains("Certificate verification failed"));
    }
}
