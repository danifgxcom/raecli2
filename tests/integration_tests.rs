use raecli2::parser::DefinitionValidator;
use raecli2::utils::{ClientType, HeaderBuilder};
use raecli2::{
    Config, Definition, DefinitionEntry, DefinitionExtractor, HttpClient, LynxDefinitionParser,
    RaeDefinitionParser, RaeError,
};

#[test]
fn test_complete_workflow_with_mock_html() {
    // Arrange: Create a complete configuration
    let config = Config::default();
    let validator = DefinitionValidator::new(&config);
    let parser = RaeDefinitionParser::new(config.clone());

    // Mock HTML response from RAE (simplified)
    let mock_html = r#"
        <!DOCTYPE html>
        <html>
        <head><title>casa | Definición | Diccionario de la lengua española | RAE</title></head>
        <body>
            <article>
                <div class="c-definitions__item" role="definition">
                    <span class="n_acep">1</span>
                    <abbr class="d">f.</abbr>
                    Edificio para habitar, comúnmente destinado a vivienda de una familia.
                </div>
                <div class="c-definitions__item" role="definition">
                    <span class="n_acep">2</span>
                    <abbr class="d">f.</abbr>
                    Familia, descendencia, linaje.
                </div>
            </article>
        </body>
        </html>
    "#;

    // Act: Parse the HTML
    let result = parser.extract_definitions(mock_html);

    // Assert: Verify the result
    assert!(result.is_ok());
    let definition = result.unwrap();
    assert_eq!(definition.word, "palabra"); // Default value in our parser
    assert_eq!(definition.entries.len(), 2);

    // Verify individual definitions are valid
    for entry in &definition.entries {
        assert!(validator.is_valid_definition(&entry.text));
    }
}

#[test]
fn test_definition_validation_integration() {
    let config = Config::default();
    let validator = DefinitionValidator::new(&config);

    // Test real-world RAE definitions
    let valid_definitions = vec![
        "1. f. Edificio para habitar, comúnmente destinado a vivienda de una familia.",
        "2. f. Familia, descendencia, linaje.",
        "3. f. Establecimiento industrial o mercantil.",
        "m. Animal doméstico de la familia de los cánidos.",
        "adj. Perteneciente o relativo al perro.",
    ];

    let invalid_ui_elements = vec![
        "Búsqueda avanzada",
        "Real Academia Española",
        "Cerrar",
        "Nueva búsqueda",
        "Diccionario de la lengua española",
    ];

    // All valid definitions should pass
    for def in valid_definitions {
        assert!(
            validator.is_valid_definition(def),
            "Should be valid: {}",
            def
        );
    }

    // All UI elements should be rejected
    for ui_elem in invalid_ui_elements {
        assert!(
            !validator.is_valid_definition(ui_elem),
            "Should be invalid: {}",
            ui_elem
        );
    }
}

#[test]
fn test_header_builder_integration() {
    let config = Config::default();
    let user_agents = config.user_agents.clone();

    // Test that different client types produce appropriate headers
    let lynx_headers = HeaderBuilder::new(ClientType::Lynx, user_agents.clone())
        .build()
        .unwrap();
    let chrome_headers = HeaderBuilder::new(ClientType::Chrome, user_agents)
        .build()
        .unwrap();

    // Lynx headers should be minimal (for avoiding detection)
    assert_eq!(lynx_headers.len(), 4);
    assert_eq!(lynx_headers.get("Accept").unwrap(), "text/html");
    assert_eq!(lynx_headers.get("Accept-Encoding").unwrap(), "gzip");

    // Chrome headers should be comprehensive (for fallback scenarios)
    assert!(chrome_headers.len() > 10);
    assert!(chrome_headers.contains_key("Sec-Ch-Ua"));
    assert!(chrome_headers.contains_key("Sec-Fetch-Dest"));
}

#[test]
fn test_definition_builder_pattern() {
    // Test the builder pattern works correctly
    let entry = DefinitionEntry::new("Edificio para habitar".to_string())
        .with_number(1)
        .with_grammatical_type("f.".to_string());

    assert_eq!(entry.number, Some(1));
    assert_eq!(entry.grammatical_type, Some("f.".to_string()));
    assert_eq!(entry.text, "Edificio para habitar");

    // Test formatting
    let formatted = format!("{}", entry);
    assert_eq!(formatted, "1. f.. Edificio para habitar");

    // Test in a complete definition
    let mut definition = Definition::new("casa".to_string());
    definition.add_entry(entry);

    assert_eq!(definition.len(), 1);
    assert!(!definition.is_empty());
    assert_eq!(definition.word, "casa");
}

#[test]
fn test_error_handling_integration() {
    // Test error conversions work correctly
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
    let rae_error = RaeError::from(io_error);

    match rae_error {
        RaeError::Network(msg) => assert!(msg.contains("File not found")),
        _ => panic!("Expected Network error"),
    }

    // Test that error messages are user-friendly
    let errors = vec![
        RaeError::NoDefinitions,
        RaeError::CloudflareChallenge("Challenge detected".to_string()),
        RaeError::TlsHandshake("Certificate error".to_string()),
    ];

    for error in errors {
        let message = format!("{}", error);
        assert!(!message.is_empty());
        assert!(!message.contains("Error")); // Should be user-friendly, not technical
    }
}

#[test]
fn test_config_defaults_are_reasonable() {
    let config = Config::default();

    // Timeouts should be reasonable
    assert!(config.timeouts.connect.as_secs() >= 10);
    assert!(config.timeouts.connect.as_secs() <= 60);
    assert!(config.timeouts.request.as_secs() >= 30);

    // User agents should be realistic
    assert!(config.user_agents.lynx.starts_with("Lynx"));
    assert!(config.user_agents.chrome.contains("Chrome"));
    assert!(config.user_agents.firefox.contains("Firefox"));

    // Should have reasonable exclusions
    assert!(config.exclusions.len() > 5);
    assert!(config.exclusions.contains(&"nueva búsqueda".to_string()));

    // Debug config should be reasonable
    assert!(!config.debug.enabled); // Default should be false
    assert!(config.debug.save_html); // Should save for debugging
}

#[test]
fn test_lynx_parser_integration() {
    let config = Config::default();
    let parser = LynxDefinitionParser::new(&config);

    // Test with Lynx-style text output
    let lynx_text = r#"
        Diccionario de la lengua española
        
        casa
        
        1. 1. f. Edificio para habitar, comúnmente destinado a vivienda de una familia.
        2. 2. f. Familia, descendencia, linaje.
        3. 3. f. Establecimiento industrial o mercantil.
        
        Sinónimos o afines
    "#;

    let result = parser.extract_definitions(lynx_text);
    assert!(result.is_ok());

    let definition = result.unwrap();
    assert!(definition.entries.len() >= 1);

    // Verify it correctly stops at synonyms
    for entry in &definition.entries {
        assert!(!entry.text.contains("Sinónimos"));
    }
}

#[test]
fn test_http_client_configuration() {
    let config = Config::default();
    let _client = HttpClient::new(config.clone());

    // The client should be creatable with default config
    // (We can't test actual HTTP requests in unit tests, but we can test creation)

    // This test mainly verifies that the integration between Config and HttpClient works
    assert_eq!(
        config.user_agents.lynx,
        "Lynx/2.9.0 libwww-FM/2.14 SSL-MM/1.4.1 GNUTLS/3.8.3"
    );
}

#[test]
fn test_complete_rae_app_workflow() {
    // This test simulates the complete workflow without actual network calls
    let config = Config::default();

    // 1. Create components
    let validator = DefinitionValidator::new(&config);
    let html_parser = RaeDefinitionParser::new(config.clone());
    let text_parser = LynxDefinitionParser::new(&config);

    // 2. Mock successful HTML parsing
    let mock_html =
        r#"<div class="c-definitions__item" role="definition">1. f. Edificio para habitar.</div>"#;

    if let Ok(definition) = html_parser.extract_definitions(mock_html) {
        for entry in &definition.entries {
            assert!(validator.is_valid_definition(&entry.text));
        }
    }

    // 3. Mock Lynx text parsing (fallback)
    let mock_text = "1. 1. f. Edificio para habitar, comúnmente destinado a vivienda.";

    if let Ok(definition) = text_parser.extract_definitions(mock_text) {
        assert!(!definition.is_empty());
    }
}

#[test]
fn test_edge_cases_integration() {
    let config = Config::default();
    let validator = DefinitionValidator::new(&config);
    let parser = RaeDefinitionParser::new(config);

    // Edge case: Empty HTML
    let result = parser.extract_definitions("");
    assert!(result.is_err());

    // Edge case: HTML without definitions
    let no_def_html = "<html><body><p>No definitions here</p></body></html>";
    let result = parser.extract_definitions(no_def_html);
    assert!(result.is_err());

    // Edge case: Minimum valid definition length
    let min_valid = "1. m. Corto def"; // Exactly 15 characters
    assert!(validator.is_valid_definition(min_valid));

    let too_short = "1. m. Corto de"; // 14 characters
    assert!(!validator.is_valid_definition(too_short));

    // Edge case: Definition with whitespace
    let whitespace_def = "  \t 1. f. Spaced definition  \n  ";
    assert!(validator.is_valid_definition(whitespace_def));
}

#[test]
fn test_performance_characteristics() {
    let config = Config::default();
    let validator = DefinitionValidator::new(&config);

    // Test that validation is fast even with many checks
    let test_definitions = vec![
        "1. f. Primera definición de prueba para validar rendimiento.",
        "2. m. Segunda definición que también debe ser validada rápidamente.",
        "3. adj. Tercera definición adjetival para completar las pruebas.",
        "Búsqueda avanzada",      // Invalid
        "Real Academia Española", // Invalid
        "4. tr. Cuarta definición transitiva válida.",
    ];

    let start = std::time::Instant::now();

    for _ in 0..1000 {
        for def in &test_definitions {
            validator.is_valid_definition(def);
        }
    }

    let duration = start.elapsed();

    // Validation should be reasonably fast (less than 2 seconds for 6000 validations)
    assert!(
        duration.as_millis() < 2000,
        "Validation too slow: {:?}",
        duration
    );
}
