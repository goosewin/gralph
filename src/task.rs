pub fn task_blocks_from_contents(contents: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut block = String::new();

    for line in contents.lines() {
        if is_task_header(line) {
            if in_block {
                blocks.push(block.clone());
                block.clear();
            }
            in_block = true;
            block.push_str(line);
            continue;
        }

        if in_block && is_task_block_end(line) {
            blocks.push(block.clone());
            block.clear();
            in_block = false;
            continue;
        }

        if in_block {
            block.push('\n');
            block.push_str(line);
        }
    }

    if in_block && !block.is_empty() {
        blocks.push(block);
    }

    blocks
}

pub fn is_task_header(line: &str) -> bool {
    line.trim_start().starts_with("### Task ")
}

pub fn is_task_block_end(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed == "---" {
        return true;
    }
    line.trim_start().starts_with("## ")
}

pub fn is_unchecked_line(line: &str) -> bool {
    line.trim_start().starts_with("- [ ]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::string::string_regex;

    fn whitespace_strategy() -> impl Strategy<Value = String> {
        string_regex(r"[ \t]{0,4}").unwrap()
    }

    fn task_id_strategy() -> impl Strategy<Value = String> {
        string_regex(r"[A-Z0-9-]{1,8}").unwrap()
    }

    fn safe_line_strategy() -> impl Strategy<Value = String> {
        string_regex(r"[A-Za-z0-9][A-Za-z0-9 .,]{0,20}").unwrap()
    }

    #[test]
    fn task_blocks_from_contents_returns_empty_when_no_blocks_exist() {
        let contents = "## Overview\n- [ ] Not a task block\n---\n";
        let blocks = task_blocks_from_contents(contents);
        assert!(blocks.is_empty());
    }

    #[test]
    fn task_blocks_from_contents_ends_on_separator_and_section_heading() {
        let contents = "### Task A\n- [ ] First\n---\n### Task B\n- [ ] Second\n## Success Criteria\n- Done\n";
        let blocks = task_blocks_from_contents(contents);

        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].contains("Task A"));
        assert!(!blocks[0].contains("---"));
        assert!(blocks[1].contains("Task B"));
        assert!(!blocks[1].contains("Success Criteria"));
    }

    #[test]
    fn task_blocks_from_contents_includes_last_block_without_separator() {
        let contents = "### Task A\n- [ ] First\n---\n### Task B\n- [ ] Second";
        let blocks = task_blocks_from_contents(contents);

        assert_eq!(blocks.len(), 2);
        assert!(blocks[1].contains("Task B"));
        assert!(blocks[1].contains("- [ ] Second"));
    }

    #[test]
    fn is_task_header_accepts_leading_whitespace() {
        assert!(is_task_header("  ### Task COV-13"));
    }

    #[test]
    fn is_unchecked_line_accepts_leading_whitespace() {
        assert!(is_unchecked_line("   - [ ] Edge case"));
    }

    #[test]
    fn is_task_block_end_detects_separators_and_headings() {
        assert!(is_task_block_end("---"));
        assert!(is_task_block_end("  ---  "));
        assert!(is_task_block_end("## Notes"));
        assert!(is_task_block_end("  ## Notes"));
        assert!(!is_task_block_end("### Task COV-13"));
    }

    #[test]
    fn is_task_block_end_rejects_non_h2_or_separator_lines() {
        assert!(!is_task_block_end("----"));
        assert!(!is_task_block_end("# Notes"));
        assert!(!is_task_block_end("##Notes"));
        assert!(!is_task_block_end("### Notes"));
        assert!(!is_task_block_end("- ---"));
    }

    #[test]
    fn is_task_header_rejects_malformed_headings() {
        assert!(!is_task_header("###Task COV-29"));
        assert!(!is_task_header("## Task COV-29"));
        assert!(!is_task_header("#### Task COV-29"));
        assert!(!is_task_header("### Tasks COV-29"));
        assert!(!is_task_header("### Task"));
    }

    proptest! {
        #[test]
        fn prop_task_blocks_from_contents_round_trip(
            prefix in prop::collection::vec(safe_line_strategy(), 0..3),
            blocks in prop::collection::vec(
                (task_id_strategy(), prop::collection::vec(safe_line_strategy(), 0..4)),
                0..4
            ),
            end_last in any::<bool>(),
            suffix in prop::collection::vec(safe_line_strategy(), 0..3)
        ) {
            let mut contents = String::new();
            if !prefix.is_empty() {
                contents.push_str(&prefix.join("\n"));
            }

            let mut expected_blocks = Vec::new();
            for (index, (id, body)) in blocks.iter().enumerate() {
                if !contents.is_empty() {
                    contents.push('\n');
                }

                let header = format!("### Task {}", id);
                contents.push_str(&header);
                let mut expected = header;

                for line in body {
                    contents.push('\n');
                    contents.push_str(line);
                    expected.push('\n');
                    expected.push_str(line);
                }

                let is_last = index + 1 == blocks.len();
                if !is_last || end_last {
                    contents.push('\n');
                    contents.push_str("---");
                }

                if is_last && end_last && !suffix.is_empty() {
                    contents.push('\n');
                    contents.push_str(&suffix.join("\n"));
                } else if !is_last {
                    contents.push('\n');
                }

                expected_blocks.push(expected);
            }

            let blocks_out = task_blocks_from_contents(&contents);
            prop_assert_eq!(blocks_out, expected_blocks);
        }

        #[test]
        fn prop_is_task_header_accepts_valid_prefix(
            leading in whitespace_strategy(),
            id in task_id_strategy(),
            tail in string_regex(r"[^\n]{0,12}").unwrap()
        ) {
            let line = format!("{}### Task {}{}", leading, id, tail);
            prop_assert!(is_task_header(&line));
        }

        #[test]
        fn prop_is_task_header_rejects_invalid_prefix(
            leading in whitespace_strategy(),
            tail in string_regex(r"[^\n]{0,12}").unwrap(),
            variant in prop_oneof![
                Just("## Task ".to_string()),
                Just("###Task ".to_string()),
                Just("#### Task ".to_string()),
                Just("### Tasks ".to_string())
            ]
        ) {
            let line = format!("{}{}{}", leading, variant, tail);
            prop_assert!(!is_task_header(&line));
        }

        #[test]
        fn prop_is_task_block_end_accepts_separator(
            leading in whitespace_strategy(),
            trailing in whitespace_strategy()
        ) {
            let line = format!("{}---{}", leading, trailing);
            prop_assert!(is_task_block_end(&line));
        }

        #[test]
        fn prop_is_task_block_end_accepts_h2_heading(
            leading in whitespace_strategy(),
            title in string_regex(r"[A-Za-z0-9][A-Za-z0-9 ]{0,12}").unwrap()
        ) {
            let line = format!("{}## {}", leading, title);
            prop_assert!(is_task_block_end(&line));
        }

        #[test]
        fn prop_is_task_block_end_rejects_non_matches(
            leading in whitespace_strategy(),
            tail in string_regex(r"[^\n]{0,12}").unwrap(),
            variant in prop_oneof![
                Just("### ".to_string()),
                Just("##".to_string()),
                Just("# ".to_string()),
                Just("----".to_string())
            ]
        ) {
            let line = format!("{}{}{}", leading, variant, tail);
            prop_assert!(!is_task_block_end(&line));
        }

        #[test]
        fn prop_is_unchecked_line_accepts_valid_prefix(
            leading in whitespace_strategy(),
            tail in string_regex(r"[^\n]{0,12}").unwrap()
        ) {
            let line = format!("{}- [ ]{}", leading, tail);
            prop_assert!(is_unchecked_line(&line));
        }

        #[test]
        fn prop_is_unchecked_line_rejects_invalid_prefix(
            leading in whitespace_strategy(),
            tail in string_regex(r"[^\n]{0,12}").unwrap(),
            variant in prop_oneof![
                Just("- [x]".to_string()),
                Just("- [X]".to_string()),
                Just("- []".to_string()),
                Just("-[ ]".to_string()),
                Just("[ ]".to_string())
            ]
        ) {
            let line = format!("{}{}{}", leading, variant, tail);
            prop_assert!(!is_unchecked_line(&line));
        }
    }
}
