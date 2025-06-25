# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

RAE CLI is a Rust command-line utility that searches for word definitions in the Spanish Royal Academy Dictionary (RAE). The tool uses headless Chrome automation to bypass Cloudflare protection and scrape definitions from dle.rae.es.

## Build and Development Commands

- **Build**: `./build.sh` or `cargo build --release`
- **Install**: `./install.sh` (copies binary to /usr/local/bin)
- **Run (Chrome method)**: `cargo run -- <word>` or `./target/release/raecli2 <word>`
- **Run (HTTP method)**: `cargo run -- --http <word>` (faster, less reliable)
- **Debug mode**: `cargo run -- --http --debug <word>` (saves HTML for analysis)
- **Test**: No test framework configured
- **Lint**: Use `cargo clippy` for linting
- **Format**: Use `cargo fmt` for code formatting

## Architecture

### Core Components

1. **Dual Scraping Engine**:
   - **Chrome Headless** (`src/main.rs:144-198`): Primary method with anti-detection measures for Cloudflare bypass
   - **HTTP Client** (`src/main.rs:64-142`): Alternative method with advanced fingerprinting and challenge solver
2. **HTML Parser** (`src/main.rs:200-265`): Uses scraper crate to extract definitions from RAE website structure
3. **Definition Validator** (`src/main.rs:267-348`): Filters real definitions from navigation/UI elements using pattern matching
4. **Cloudflare Challenge Handler** (`src/main.rs:350-409`): Detects and attempts to resolve basic Cloudflare challenges
5. **Color Display** (`src/main.rs:8-25`): Colorizes output using the colored crate

### Anti-Detection Strategies

**Chrome Headless Method:**
- JavaScript injection to hide webdriver properties
- Custom user agent and browser arguments
- Navigator property manipulation to appear as a real browser
- Timing delays to simulate human behavior

**HTTP Client Method:**
- Realistic browser fingerprinting with Chrome user-agent
- Two-step navigation simulation (homepage → search)
- Proper Sec-Fetch headers and referrer management
- Cloudflare challenge detection and 5-second wait handling
- **Node.js JavaScript execution**: Automatically extracts and solves JavaScript challenges
- Cookie persistence across requests

### Definition Extraction Logic

Uses multiple CSS selectors with fallback strategy:
- Primary: `article p`, `article ol li`, `article ul li`
- Secondary: `.normal p`, `.normal li`, `#resultados p`, `#resultados li`
- Validates definitions by checking for grammatical categories (m., f., adj., etc.)
- Filters out navigation elements and website UI text

## Dependencies

- **reqwest**: HTTP client with blocking and cookies support
- **clap**: Command-line argument parsing with derive macros
- **scraper**: HTML parsing and CSS selector support
- **headless_chrome**: Browser automation for anti-detection scraping
- **colored**: Terminal color output
- **urlencoding**: URL encoding for search terms

## System Requirements

- **Chrome/Chromium**: Required for headless method
- **Node.js**: Required for JavaScript challenge solving in HTTP method

## Key Technical Notes

- Target URL format: `https://dle.rae.es/{encoded_word}`
- Waits up to 15 seconds for page load
- Definition validation requires minimum 15 characters and grammatical markers
- Binary name is `raecli2` but project references "rae-cli" in clap configuration