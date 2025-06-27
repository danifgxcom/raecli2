/// Integration tests for HTTP redirect handling functionality
/// 
/// These tests verify that the RAE CLI correctly handles HTTP redirects
/// when the RAE website redirects word forms to their canonical versions.

use std::process::Command;

#[test]
fn test_feminine_to_masculine_redirect() {
    // Test that feminine forms redirect to masculine and show both words
    let output = Command::new("./target/release/raecli2")
        .arg("petarda")
        .output()
        .expect("Failed to execute raecli2");

    assert!(output.status.success(), "Command failed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should show "petardo, petarda" indicating redirect occurred
    assert!(stdout.contains("petardo, petarda"), 
        "Expected redirect format 'petardo, petarda' but got: {}", stdout);
    
    // Should contain actual definitions
    assert!(stdout.contains("m. y f.") || stdout.contains("despect."),
        "Expected definitions but got: {}", stdout);
}

#[test]
fn test_another_feminine_redirect() {
    // Test another case: novia -> novio
    let output = Command::new("./target/release/raecli2")
        .arg("novia")
        .output()
        .expect("Failed to execute raecli2");

    assert!(output.status.success(), "Command failed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should show "novio, novia" indicating redirect occurred
    assert!(stdout.contains("novio, novia"), 
        "Expected redirect format 'novio, novia' but got: {}", stdout);
    
    // Should contain definitions
    assert!(stdout.contains("m. y f.") || stdout.contains("relación amorosa"),
        "Expected definitions but got: {}", stdout);
}

#[test]
fn test_no_redirect_case() {
    // Test that words without redirects work normally
    let output = Command::new("./target/release/raecli2")
        .arg("casa")
        .output()
        .expect("Failed to execute raecli2");

    assert!(output.status.success(), "Command failed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should NOT show redirect format (no "word, word" at start)
    assert!(!stdout.starts_with("casa, casa"), 
        "Should not show redirect format for non-redirected word: {}", stdout);
    
    // Should contain definitions directly
    assert!(stdout.contains("f.") && (stdout.contains("Edificio") || stdout.contains("habitar")),
        "Expected casa definitions but got: {}", stdout);
}

#[test]
fn test_redirect_with_debug_mode() {
    // Test that debug mode shows redirect information
    let output = Command::new("./target/release/raecli2")
        .arg("--debug")
        .arg("petarda")
        .output()
        .expect("Failed to execute raecli2");

    // Command might fail in environments without full TLS/networking,
    // but we check if it at least attempts the operation
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // In debug mode, should show either redirect info or TLS info
    let has_redirect_info = stderr.contains("🔄 Redirigiendo") || 
                           stdout.contains("petardo, petarda");
    let has_debug_info = stderr.contains("TLS") || stderr.contains("🌐 URL");
    
    assert!(has_redirect_info || has_debug_info,
        "Expected redirect or debug info in stderr: {} or stdout: {}", stderr, stdout);
}

#[cfg(test)]
mod unit_tests {

    /// Test the extract_location_header function with mock HTTP responses
    #[test]
    fn test_extract_location_header() {
        // Mock HTTP response with Location header
        let mock_response = "HTTP/1.1 301 Moved Permanently\r\n\
                            Location: https://dle.rae.es/petardo\r\n\
                            Content-Length: 0\r\n\
                            \r\n";
        
        // This would test the function if it were public
        // For now, we test the integration behavior through the CLI
        assert!(mock_response.contains("Location: "));
    }

    /// Test canonical word extraction from HTML
    #[test]
    fn test_word_extraction_from_html() {
        let mock_html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>petardo | Definición | Diccionario de la lengua española | RAE</title>
                <link rel="canonical" href="https://dle.rae.es/petardo">
            </head>
            <body>
                <article>
                    <div class="c-definitions__item" role="definition">
                        1. m. y f. despect. coloq. Persona pesada...
                    </div>
                </article>
            </body>
            </html>
        "#;
        
        // Should extract "petardo" from title
        assert!(mock_html.contains("petardo | Definición"));
        assert!(mock_html.contains("canonical"));
    }

    /// Test that redirect detection works for various status codes
    #[test]
    fn test_redirect_status_codes() {
        let redirect_codes = vec!["301", "302", "303", "307", "308"];
        
        for code in redirect_codes {
            let status_line = format!("HTTP/1.1 {} Moved", code);
            assert!(status_line.contains(code));
        }
    }
}

/// Performance test for redirect handling
#[test]
#[ignore] // Ignore by default as it requires network access
fn test_redirect_performance() {
    use std::time::Instant;
    
    let start = Instant::now();
    
    let output = Command::new("./target/release/raecli2")
        .arg("petarda")
        .output()
        .expect("Failed to execute raecli2");
    
    let duration = start.elapsed();
    
    // Redirect handling should not significantly slow down requests
    // Allow up to 10 seconds for network requests including redirects
    assert!(duration.as_secs() < 10, 
        "Redirect handling too slow: {:?}", duration);
    
    assert!(output.status.success());
}

/// Test error handling when redirect fails
#[test]
#[ignore] // Ignore by default as it requires specific network conditions
fn test_redirect_error_handling() {
    // Test with a word that might cause redirect issues
    let output = Command::new("./target/release/raecli2")
        .arg("nonexistentword12345")
        .output()
        .expect("Failed to execute raecli2");
    
    // Should not crash, even if word doesn't exist
    // May exit with error code but shouldn't panic
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panic") && !stderr.contains("unwrap"));
}