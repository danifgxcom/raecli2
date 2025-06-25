use crate::config::Config;
use crate::parser::DefinitionValidator;
use crate::types::{Definition, DefinitionEntry, RaeError, Result};
use scraper::{Html, Selector};

pub trait DefinitionExtractor {
    fn extract_definitions(&self, html: &str) -> Result<Definition>;
}

pub struct RaeDefinitionParser {
    config: Config,
    validator: DefinitionValidator,
}

impl RaeDefinitionParser {
    pub fn new(config: Config) -> Self {
        let validator = DefinitionValidator::new(&config);
        Self { config, validator }
    }

    fn extract_with_selectors(&self, document: &Html, word: &str) -> Result<Definition> {
        let mut definition = Definition::new(word.to_string());

        for selector_str in &self.config.selectors.definition_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    if let Some(entry) = self.parse_definition_element(&element) {
                        if self.validator.is_valid_definition(&entry.text) {
                            definition.add_entry(entry);
                        }
                    }
                }

                if !definition.is_empty() {
                    return Ok(definition);
                }
            }
        }

        if definition.is_empty() {
            Err(RaeError::NoDefinitions)
        } else {
            Ok(definition)
        }
    }

    fn parse_definition_element(&self, element: &scraper::ElementRef) -> Option<DefinitionEntry> {
        let text = self.extract_clean_text(element);

        if text.trim().is_empty() {
            return None;
        }

        let mut entry = DefinitionEntry::new(text);

        // Try to extract number
        if let Ok(num_selector) = Selector::parse(&self.config.selectors.number_selector) {
            if let Some(num_element) = element.select(&num_selector).next() {
                let num_text = num_element.text().collect::<String>();
                if let Ok(number) = num_text.trim().parse::<u32>() {
                    entry = entry.with_number(number);
                }
            }
        }

        // Try to extract grammatical type
        if let Ok(abbr_selector) = Selector::parse(&self.config.selectors.abbreviation_selector) {
            if let Some(abbr_element) = element.select(&abbr_selector).next() {
                let abbr_text = abbr_element.text().collect::<String>().trim().to_string();
                if !abbr_text.is_empty() {
                    entry = entry.with_grammatical_type(abbr_text);
                }
            }
        }

        Some(entry)
    }

    fn extract_clean_text(&self, element: &scraper::ElementRef) -> String {
        element
            .text()
            .collect::<String>()
            .lines()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join(" ")
            .replace("  ", " ")
            .trim()
            .to_string()
    }
}

impl DefinitionExtractor for RaeDefinitionParser {
    fn extract_definitions(&self, html: &str) -> Result<Definition> {
        let document = Html::parse_document(html);

        // Extract word from URL or use placeholder
        let word = "palabra"; // This could be extracted from context

        self.extract_with_selectors(&document, word)
    }
}

pub struct LynxDefinitionParser {
    validator: DefinitionValidator,
}

impl LynxDefinitionParser {
    pub fn new(config: &Config) -> Self {
        let validator = DefinitionValidator::new(config);
        Self { validator }
    }

    fn extract_from_text(&self, content: &str, word: &str) -> Result<Definition> {
        let mut definition = Definition::new(word.to_string());
        let mut in_definition_section = false;

        for line in content.lines() {
            let line_trimmed = line.trim();

            // Detect start of definitions section
            if line_trimmed.contains("Diccionario de la lengua española") {
                in_definition_section = true;
                continue;
            }

            if in_definition_section {
                // Stop at compound phrases or synonyms
                if self.should_stop_parsing(line_trimmed) {
                    break;
                }

                // Extract definition lines
                if let Some(entry) = self.parse_definition_line(line_trimmed) {
                    if self.validator.is_valid_definition(&entry.text) {
                        definition.add_entry(entry);
                    }
                }
            }
        }

        if definition.is_empty() {
            Err(RaeError::NoDefinitions)
        } else {
            Ok(definition)
        }
    }

    fn should_stop_parsing(&self, line: &str) -> bool {
        line.starts_with("Sinónimos o afines")
            || line.starts_with("Antónimos u opuestos")
            || line.starts_with("Palabra del día")
            || (line.contains(' ')
                && !line.starts_with("f.")
                && !line.starts_with("m.")
                && !line.starts_with("adj.")
                && !line.starts_with("adv.")
                && !line.starts_with("tr.")
                && !line.chars().next().is_some_and(|c| c.is_ascii_digit()))
    }

    fn parse_definition_line(&self, line: &str) -> Option<DefinitionEntry> {
        if line.len() < 6 {
            return None;
        }

        // Pattern: "X. Y. text" (e.g., "1. 1. f. Definition text")
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let first = parts[0];
            let second = parts[1];

            if first.ends_with('.')
                && second.ends_with('.')
                && first.chars().all(|c| c.is_ascii_digit() || c == '.')
                && second.chars().all(|c| c.is_ascii_digit() || c == '.')
            {
                return Some(DefinitionEntry::new(line.to_string()));
            }
        }

        None
    }
}

impl DefinitionExtractor for LynxDefinitionParser {
    fn extract_definitions(&self, content: &str) -> Result<Definition> {
        // Determine if content is HTML or plain text
        if content.starts_with("HTTP/1.1") || content.contains("<!DOCTYPE html>") {
            // Parse as HTML first, then fallback to text
            let document = Html::parse_document(content);
            let word = "palabra"; // Extract from context

            // Try main content selectors
            if let Ok(selector) = Selector::parse("div[role='definition']") {
                let mut definition = Definition::new(word.to_string());

                for element in document.select(&selector) {
                    let text = element.text().collect::<String>();
                    let clean_text = text
                        .lines()
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<&str>>()
                        .join(" ");

                    if self.validator.is_valid_definition(&clean_text) {
                        definition.add_entry(DefinitionEntry::new(clean_text));
                    }
                }

                if !definition.is_empty() {
                    return Ok(definition);
                }
            }
        }

        // Fallback to text parsing
        self.extract_from_text(content, "palabra")
    }
}
