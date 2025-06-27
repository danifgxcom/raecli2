# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

RAE CLI is a Rust command-line utility that searches for word definitions in the Spanish Royal Academy Dictionary (RAE). The tool uses TLS fingerprinting to mimic the Lynx browser and bypass Cloudflare protection.

## Build and Development Commands

- **Build**: `./build.sh` or `cargo build --release`
- **Install**: `./install.sh` (copies binary to /usr/local/bin)
- **Run**: `cargo run -- <word>` or `./target/release/raecli2 <word>`
- **Debug mode**: `cargo run -- --debug <word>` (saves HTML and TLS logs for analysis)
- **Test**: `cargo test` (integration tests available)
- **Redirect Tests**: `cargo test --test redirect_tests` (specific tests for redirect functionality)
- **Lint**: Use `cargo clippy` for linting
- **Format**: Use `cargo fmt` for code formatting

## Architecture

### Core Components

1. **TLS Fingerprint Client** (`src/main.rs:1502-1831`): Custom TLS implementation that mimics Lynx browser exactly to bypass Cloudflare
2. **HTTP Fallback Client** (`src/main.rs:104-169`): Simple reqwest-based fallback with Lynx user-agent
3. **HTML Parser** (`src/main.rs:171-305`): Uses scraper crate to extract definitions from RAE website structure
4. **Definition Validator** (`src/main.rs:307-378`): Filters real definitions from navigation/UI elements using pattern matching
5. **Cloudflare Detection** (`src/main.rs:380-433`): Detects Cloudflare challenge pages
6. **Color Display** (`src/main.rs:27-45`): Colorizes output using the colored crate
7. **HTTP Redirect Handler** (`src/main.rs:1574-1594`, `src/main.rs:1611-1689`): Automatically follows HTTP redirections for canonical word forms

### TLS Fingerprinting Strategy

**Custom TLS Implementation:**
- Direct rustls usage with exact Lynx configuration
- Manual HTTP/1.1 request construction
- Precise timing simulation (150ms + 200ms delays)
- Raw TCP socket handling
- Certificate and cipher suite logging for debugging

**Fallback HTTP Method:**
- Lynx user-agent: "Lynx/2.9.0 libwww-FM/2.14 SSL-MM/1.4.1 GNUTLS/3.8.3"
- Minimal headers (Accept: text/html, Accept-Language: es)
- Basic Cloudflare challenge detection
- Automatic redirect following (up to 10 redirects)

### Definition Extraction Logic

Uses multiple CSS selectors with fallback strategy:
- Primary: `.c-definitions__item[role='definition']`, `.c-definitions li`
- Secondary: `article p`, `article ol li`, `article ul li`
- Validates definitions by checking for grammatical categories (m., f., adj., etc.)
- Filters out navigation elements and website UI text

## Dependencies

- **reqwest**: HTTP client with blocking and cookies support (fallback method)
- **clap**: Command-line argument parsing with derive macros
- **scraper**: HTML parsing and CSS selector support
- **colored**: Terminal color output
- **urlencoding**: URL encoding for search terms
- **rustls**: Custom TLS implementation for fingerprinting
- **webpki-roots**: Root certificates for TLS
- **flate2**: Gzip decompression
- **encoding_rs**: Character encoding detection
- **url**: URL parsing

## System Requirements

- **No external dependencies**: Self-contained TLS implementation

## Key Technical Notes

- Target URL format: `https://dle.rae.es/{encoded_word}`
- TLS fingerprinting bypasses Cloudflare detection
- Definition validation requires minimum 15 characters and grammatical markers
- Binary name is `raecli2` but project references "rae-cli" in clap configuration
- Debug mode saves HTML files and TLS handshake logs for analysis

## HTTP Redirect Handling Challenge

### Problem Statement
The RAE website automatically redirects certain word forms to their canonical base forms using HTTP 301/302 redirects. For example:
- Feminine forms redirect to masculine: `petarda` → `petardo`
- Derived forms redirect to base: `novia` → `novio`
- Irregular forms redirect to dictionary headwords

Without redirect handling, the CLI would fail to show definitions for redirected terms.

### Technical Solution

**TLS Fingerprint Client:**
- Manual HTTP status line parsing to detect 301/302/303/307/308 responses
- Location header extraction from raw HTTP response
- Recursive calls to `lynx_tls_fingerprint_con_host()` for same-domain redirects
- Limited to `dle.rae.es` domain for security

**HTTP Fallback Client:**
- Built-in redirect support via `reqwest::redirect::Policy::limited(10)`
- Automatic handling without manual intervention

**Word Extraction:**
- Parse final canonical word from HTML `<title>` tag or `link[rel='canonical']`
- Display format: `canonical_word, searched_word` when different
- Preserves user context while showing actual dictionary entry

### Implementation Details

Key functions:
- `extract_location_header()`: Parses HTTP Location header
- `extraer_palabra_de_respuesta()`: Extracts canonical word from HTML
- `lynx_tls_fingerprint_con_host()`: Handles TLS redirect requests

**Example Output:**
```
$ raecli2 petarda
petardo, petarda

1. m. y f. despect. coloq. Persona pesada, aburrida...
```

### Test Cases
- `petarda` → `petardo, petarda` + definitions
- `novia` → `novio, novia` + definitions  
- `casa` → definitions only (no redirect)

### Automated Tests
The redirect functionality is covered by integration tests in `tests/redirect_tests.rs`:

**Test Coverage:**
- Feminine-to-masculine redirects (`test_feminine_to_masculine_redirect`)
- Alternative redirect cases (`test_another_feminine_redirect`) 
- Non-redirect scenarios (`test_no_redirect_case`)
- Debug mode behavior (`test_redirect_with_debug_mode`)
- Unit tests for HTTP parsing logic

**Run Tests:**
```bash
cargo test --test redirect_tests
```

**Performance Tests:**
- Redirect response time validation (ignored by default)
- Error handling for invalid words (ignored by default)