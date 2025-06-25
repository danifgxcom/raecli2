use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Definition {
    pub word: String,
    pub entries: Vec<DefinitionEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DefinitionEntry {
    pub number: Option<u32>,
    pub grammatical_type: Option<String>,
    pub text: String,
}

impl Definition {
    pub fn new(word: String) -> Self {
        Self {
            word,
            entries: Vec::new(),
        }
    }

    pub fn add_entry(&mut self, entry: DefinitionEntry) {
        self.entries.push(entry);
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

impl DefinitionEntry {
    pub fn new(text: String) -> Self {
        Self {
            number: None,
            grammatical_type: None,
            text,
        }
    }

    pub fn with_number(mut self, number: u32) -> Self {
        self.number = Some(number);
        self
    }

    pub fn with_grammatical_type(mut self, grammatical_type: String) -> Self {
        self.grammatical_type = Some(grammatical_type);
        self
    }
}

impl fmt::Display for Definition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for entry in &self.entries {
            writeln!(f, "{}", entry)?;
        }
        Ok(())
    }
}

impl fmt::Display for DefinitionEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(number) = self.number {
            write!(f, "{}. ", number)?;
        }
        if let Some(ref grammatical_type) = self.grammatical_type {
            write!(f, "{}. ", grammatical_type)?;
        }
        write!(f, "{}", self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_definition_new() {
        let definition = Definition::new("casa".to_string());
        assert_eq!(definition.word, "casa");
        assert!(definition.is_empty());
        assert_eq!(definition.len(), 0);
    }

    #[test]
    fn test_definition_add_entry() {
        let mut definition = Definition::new("casa".to_string());
        let entry = DefinitionEntry::new("Edificio para habitar".to_string());

        definition.add_entry(entry.clone());

        assert!(!definition.is_empty());
        assert_eq!(definition.len(), 1);
        assert_eq!(definition.entries[0], entry);
    }

    #[test]
    fn test_definition_multiple_entries() {
        let mut definition = Definition::new("casa".to_string());

        definition.add_entry(DefinitionEntry::new("Primera definición".to_string()));
        definition.add_entry(DefinitionEntry::new("Segunda definición".to_string()));

        assert_eq!(definition.len(), 2);
        assert_eq!(definition.entries[0].text, "Primera definición");
        assert_eq!(definition.entries[1].text, "Segunda definición");
    }

    #[test]
    fn test_definition_entry_new() {
        let entry = DefinitionEntry::new("Texto de definición".to_string());

        assert_eq!(entry.text, "Texto de definición");
        assert_eq!(entry.number, None);
        assert_eq!(entry.grammatical_type, None);
    }

    #[test]
    fn test_definition_entry_with_number() {
        let entry = DefinitionEntry::new("Texto".to_string()).with_number(1);

        assert_eq!(entry.number, Some(1));
        assert_eq!(entry.text, "Texto");
    }

    #[test]
    fn test_definition_entry_with_grammatical_type() {
        let entry =
            DefinitionEntry::new("Texto".to_string()).with_grammatical_type("f.".to_string());

        assert_eq!(entry.grammatical_type, Some("f.".to_string()));
        assert_eq!(entry.text, "Texto");
    }

    #[test]
    fn test_definition_entry_builder_pattern() {
        let entry = DefinitionEntry::new("Edificio para habitar".to_string())
            .with_number(1)
            .with_grammatical_type("m.".to_string());

        assert_eq!(entry.number, Some(1));
        assert_eq!(entry.grammatical_type, Some("m.".to_string()));
        assert_eq!(entry.text, "Edificio para habitar");
    }

    #[test]
    fn test_definition_entry_display_minimal() {
        let entry = DefinitionEntry::new("Solo texto".to_string());
        assert_eq!(format!("{}", entry), "Solo texto");
    }

    #[test]
    fn test_definition_entry_display_with_number() {
        let entry = DefinitionEntry::new("Con número".to_string()).with_number(2);
        assert_eq!(format!("{}", entry), "2. Con número");
    }

    #[test]
    fn test_definition_entry_display_with_grammatical_type() {
        let entry = DefinitionEntry::new("Con tipo gramatical".to_string())
            .with_grammatical_type("adj.".to_string());
        assert_eq!(format!("{}", entry), "adj.. Con tipo gramatical");
    }

    #[test]
    fn test_definition_entry_display_complete() {
        let entry = DefinitionEntry::new("Definición completa".to_string())
            .with_number(1)
            .with_grammatical_type("f.".to_string());
        assert_eq!(format!("{}", entry), "1. f.. Definición completa");
    }

    #[test]
    fn test_definition_display() {
        let mut definition = Definition::new("casa".to_string());
        definition.add_entry(
            DefinitionEntry::new("Primera definición".to_string())
                .with_number(1)
                .with_grammatical_type("f.".to_string()),
        );
        definition.add_entry(
            DefinitionEntry::new("Segunda definición".to_string())
                .with_number(2)
                .with_grammatical_type("f.".to_string()),
        );

        let expected = "1. f.. Primera definición\n2. f.. Segunda definición\n";
        assert_eq!(format!("{}", definition), expected);
    }

    #[test]
    fn test_definition_equality() {
        let mut def1 = Definition::new("casa".to_string());
        let mut def2 = Definition::new("casa".to_string());

        let entry = DefinitionEntry::new("Edificio".to_string()).with_number(1);
        def1.add_entry(entry.clone());
        def2.add_entry(entry);

        assert_eq!(def1, def2);
    }

    #[test]
    fn test_definition_inequality() {
        let def1 = Definition::new("casa".to_string());
        let def2 = Definition::new("perro".to_string());

        assert_ne!(def1, def2);
    }
}
