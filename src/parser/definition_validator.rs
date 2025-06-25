use crate::config::Config;

pub struct DefinitionValidator {
    exclusions: Vec<String>,
    grammatical_categories: Vec<String>,
    grammatical_starters: Vec<String>,
}

impl DefinitionValidator {
    pub fn new(config: &Config) -> Self {
        Self {
            exclusions: config.exclusions.clone(),
            grammatical_categories: vec![
                "m.", "f.", "adj.", "tr.", "intr.", "prnl.", "loc.", "adv.", "interj.", "prep.",
                "conj.",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            grammatical_starters: vec![
                "m. ", "f. ", "adj. ", "tr. ", "intr. ", "prnl. ", "loc. ", "adv. ", "interj. ",
                "prep. ", "conj. ", "m. y f. ",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }

    pub fn is_valid_definition(&self, text: &str) -> bool {
        let text = text.trim();

        // Must have minimum reasonable length
        if text.len() < 15 {
            return false;
        }

        // Check for exclusions (navigation and UI elements)
        if self.contains_exclusions(text) {
            return false;
        }

        // Check for typical RAE definition patterns
        self.has_definition_pattern(text)
    }

    fn contains_exclusions(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();
        self.exclusions
            .iter()
            .any(|exclusion| text_lower.contains(exclusion))
    }

    fn has_definition_pattern(&self, text: &str) -> bool {
        // Pattern 1: Starts with number and grammatical category: "1. m.", "2. f.", etc.
        if self.has_numbered_definition_pattern(text) {
            return true;
        }

        // Pattern 2: Starts directly with grammatical category
        if self.has_grammatical_starter_pattern(text) {
            return true;
        }

        // Pattern 3: Contains grammatical categories (more permissive)
        if self.contains_grammatical_categories(text) {
            return true;
        }

        false
    }

    fn has_numbered_definition_pattern(&self, text: &str) -> bool {
        if let Some(first_char) = text.chars().next() {
            if first_char.is_ascii_digit() && text.contains(". ") {
                return self
                    .grammatical_categories
                    .iter()
                    .any(|category| text.contains(category));
            }
        }
        false
    }

    fn has_grammatical_starter_pattern(&self, text: &str) -> bool {
        self.grammatical_starters
            .iter()
            .any(|starter| text.starts_with(starter))
    }

    fn contains_grammatical_categories(&self, text: &str) -> bool {
        self.grammatical_categories
            .iter()
            .any(|category| text.contains(category))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> Config {
        Config::default()
    }

    fn create_validator() -> DefinitionValidator {
        DefinitionValidator::new(&create_test_config())
    }

    #[test]
    fn test_validator_creation() {
        let validator = create_validator();
        // Validator should be created successfully
        assert!(validator.exclusions.len() > 0);
        assert!(validator.grammatical_categories.len() > 0);
        assert!(validator.grammatical_starters.len() > 0);
    }

    #[test]
    fn test_valid_definition_with_number_and_category() {
        let validator = create_validator();
        let text = "1. f. Edificio para habitar, comúnmente destinado a vivienda de una familia.";

        assert!(validator.is_valid_definition(text));
    }

    #[test]
    fn test_valid_definition_with_grammatical_starter() {
        let validator = create_validator();
        let text = "f. Edificio para habitar, comúnmente destinado a vivienda de una familia.";

        assert!(validator.is_valid_definition(text));
    }

    #[test]
    fn test_valid_definition_with_multiple_categories() {
        let validator = create_validator();
        let text = "2. adj. Que pertenece a la casa o es propio de ella.";

        assert!(validator.is_valid_definition(text));
    }

    #[test]
    fn test_invalid_definition_too_short() {
        let validator = create_validator();
        let text = "Muy corto";

        assert!(!validator.is_valid_definition(text));
    }

    #[test]
    fn test_invalid_definition_empty() {
        let validator = create_validator();
        let text = "";

        assert!(!validator.is_valid_definition(text));
    }

    #[test]
    fn test_invalid_definition_whitespace_only() {
        let validator = create_validator();
        let text = "   \t\n   ";

        assert!(!validator.is_valid_definition(text));
    }

    #[test]
    fn test_invalid_definition_with_exclusions() {
        let validator = create_validator();
        let texts = vec![
            "Nueva búsqueda en el diccionario",
            "Real Academia Española - Diccionario",
            "Búsqueda avanzada para encontrar palabras",
            "Fundación de la lengua española",
            "Cerrar esta ventana de búsqueda",
        ];

        for text in texts {
            assert!(
                !validator.is_valid_definition(text),
                "Text should be invalid: {}",
                text
            );
        }
    }

    #[test]
    fn test_invalid_definition_no_grammatical_pattern() {
        let validator = create_validator();
        let text = "Este es un texto largo sin categorías gramaticales que debería ser rechazado.";

        assert!(!validator.is_valid_definition(text));
    }

    #[test]
    fn test_numbered_definition_pattern() {
        let validator = create_validator();

        // Valid numbered patterns
        assert!(validator.has_numbered_definition_pattern("1. m. Una definición válida"));
        assert!(validator.has_numbered_definition_pattern("2. f. Otra definición válida"));
        assert!(validator.has_numbered_definition_pattern("3. adj. Definición adjetival"));

        // Invalid patterns
        assert!(!validator.has_numbered_definition_pattern("No empieza con número"));
        assert!(!validator.has_numbered_definition_pattern("1 Sin punto después del número"));
        assert!(!validator.has_numbered_definition_pattern("1. Sin categoría gramatical"));
    }

    #[test]
    fn test_grammatical_starter_pattern() {
        let validator = create_validator();

        // Valid starters
        assert!(validator.has_grammatical_starter_pattern("m. Sustantivo masculino"));
        assert!(validator.has_grammatical_starter_pattern("f. Sustantivo femenino"));
        assert!(validator.has_grammatical_starter_pattern("adj. Adjetivo"));
        assert!(validator.has_grammatical_starter_pattern("m. y f. Común en cuanto al género"));

        // Invalid starters
        assert!(!validator.has_grammatical_starter_pattern("No empieza con categoría"));
        assert!(!validator.has_grammatical_starter_pattern("ma. Categoría inválida"));
        assert!(!validator.has_grammatical_starter_pattern("f Falta el punto"));
    }

    #[test]
    fn test_contains_exclusions() {
        let validator = create_validator();

        // Should be excluded
        assert!(validator.contains_exclusions("nueva búsqueda"));
        assert!(validator.contains_exclusions("REAL ACADEMIA ESPAÑOLA"));
        assert!(validator.contains_exclusions("Texto con acceso restringido"));

        // Should not be excluded
        assert!(!validator.contains_exclusions("Una definición normal"));
        assert!(!validator.contains_exclusions("1. f. Definición válida"));
    }

    #[test]
    fn test_contains_grammatical_categories() {
        let validator = create_validator();

        // Should contain categories
        assert!(validator.contains_grammatical_categories("1. m. Definición"));
        assert!(validator.contains_grammatical_categories("Text with f. in middle"));
        assert!(validator.contains_grammatical_categories("adj. Adjective definition"));

        // Should not contain categories
        assert!(!validator.contains_grammatical_categories("No grammatical markers here"));
        assert!(!validator.contains_grammatical_categories("Just regular text"));
    }

    #[test]
    fn test_edge_cases() {
        let validator = create_validator();

        // Edge case: exactly 15 characters with valid pattern
        let text = "1. m. Corto def"; // exactly 15 chars
        assert_eq!(text.len(), 15);
        assert!(validator.is_valid_definition(text));

        // Edge case: 14 characters (too short)
        let text_short = "1. m. Corto de"; // 14 chars
        assert_eq!(text_short.len(), 14);
        assert!(!validator.is_valid_definition(text_short));
    }

    #[test]
    fn test_case_insensitive_exclusions() {
        let validator = create_validator();

        // Test that exclusions work regardless of case
        assert!(!validator.is_valid_definition("NUEVA BÚSQUEDA en el sitio"));
        assert!(!validator.is_valid_definition("Nueva Búsqueda en el sitio"));
        assert!(!validator.is_valid_definition("nueva búsqueda en el sitio"));
    }

    #[test]
    fn test_real_world_definitions() {
        let validator = create_validator();

        // Real RAE-style definitions that should be valid
        let valid_definitions = vec![
            "1. f. Edificio para habitar, comúnmente destinado a vivienda de una familia.",
            "2. f. Familia, descendencia, linaje.",
            "3. f. Establecimiento industrial o mercantil.",
            "m. Animal doméstico de la familia de los cánidos.",
            "adj. Perteneciente o relativo al perro.",
        ];

        for def in valid_definitions {
            assert!(
                validator.is_valid_definition(def),
                "Should be valid: {}",
                def
            );
        }

        // UI elements that should be invalid
        let invalid_ui_elements = vec![
            "Búsqueda avanzada",
            "Cerrar",
            "Imprimir",
            "Compartir en redes sociales",
            "© Real Academia Española",
            "Diccionario de la lengua española",
        ];

        for ui_elem in invalid_ui_elements {
            assert!(
                !validator.is_valid_definition(ui_elem),
                "Should be invalid: {}",
                ui_elem
            );
        }
    }
}
