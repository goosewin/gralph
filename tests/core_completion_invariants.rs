use gralph::core::check_completion;
use proptest::prelude::*;
use proptest::string::string_regex;
use std::fs;

fn safe_line_strategy() -> impl Strategy<Value = String> {
    string_regex(r"[A-Za-z0-9][A-Za-z0-9 .,]{0,12}").unwrap()
}

fn whitespace_line_strategy() -> impl Strategy<Value = String> {
    string_regex(r"[ \t]{0,6}").unwrap()
}

fn negation_phrase_strategy() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("cannot"),
        Just("can't"),
        Just("won't"),
        Just("will not"),
        Just("do not"),
        Just("don't"),
        Just("should not"),
        Just("shouldn't"),
        Just("must not"),
        Just("mustn't"),
    ]
}

proptest! {
    #[test]
    fn completion_accepts_trailing_whitespace_after_marker(
        prefix in prop::collection::vec(safe_line_strategy(), 0..3),
        marker in string_regex(r"[A-Z]{3,8}").unwrap(),
        trailing in prop::collection::vec(whitespace_line_strategy(), 0..3),
    ) {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [x] Done\n").unwrap();

        let mut result = String::new();
        for line in prefix {
            result.push_str(&line);
            result.push('\n');
        }
        result.push_str(&format!("<promise>{}</promise>\n", marker));
        for line in trailing {
            result.push_str(&line);
            result.push('\n');
        }

        let complete = check_completion(&path, &result, &marker).unwrap();
        prop_assert!(complete);
    }

    #[test]
    fn completion_rejects_negated_last_non_empty_line(
        prefix in prop::collection::vec(safe_line_strategy(), 0..3),
        marker in string_regex(r"[A-Z]{3,8}").unwrap(),
        phrase in negation_phrase_strategy(),
        uppercase in any::<bool>(),
        trailing in prop::collection::vec(whitespace_line_strategy(), 0..3),
    ) {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [x] Done\n").unwrap();

        let phrase = if uppercase {
            phrase.to_uppercase()
        } else {
            phrase.to_string()
        };

        let mut result = String::new();
        for line in prefix {
            result.push_str(&line);
            result.push('\n');
        }
        result.push_str(&format!("<promise>{}</promise>\n", marker));
        result.push_str(&format!("{} <promise>{}</promise>\n", phrase, marker));
        for line in trailing {
            result.push_str(&line);
            result.push('\n');
        }

        let complete = check_completion(&path, &result, &marker).unwrap();
        prop_assert!(!complete);
    }
}
