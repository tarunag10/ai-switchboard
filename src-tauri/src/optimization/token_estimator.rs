use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TokenEstimate {
    pub(crate) characters: usize,
    pub(crate) estimated_tokens: u64,
}

pub(crate) fn estimate_text_tokens(text: &str) -> TokenEstimate {
    let characters = text.chars().count();

    TokenEstimate {
        characters,
        estimated_tokens: estimate_character_tokens(characters),
    }
}

pub(crate) fn estimate_character_tokens(characters: usize) -> u64 {
    if characters == 0 {
        return 0;
    }

    characters.div_ceil(4) as u64
}

#[cfg(test)]
mod tests {
    use super::{estimate_character_tokens, estimate_text_tokens};

    #[test]
    fn estimates_zero_for_empty_text() {
        let estimate = estimate_text_tokens("");

        assert_eq!(estimate.characters, 0);
        assert_eq!(estimate.estimated_tokens, 0);
    }

    #[test]
    fn estimates_using_ceil_four_character_bucket() {
        assert_eq!(estimate_character_tokens(1), 1);
        assert_eq!(estimate_character_tokens(4), 1);
        assert_eq!(estimate_character_tokens(5), 2);
    }

    #[test]
    fn counts_unicode_characters_not_bytes() {
        let estimate = estimate_text_tokens("ééééé");

        assert_eq!(estimate.characters, 5);
        assert_eq!(estimate.estimated_tokens, 2);
    }
}
