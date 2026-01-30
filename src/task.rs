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
    let trimmed_start = line.trim_start();
    let Some(rest) = trimmed_start.strip_prefix("## ") else {
        return false;
    };
    !rest.trim().is_empty()
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

    fn tab_prefix_strategy() -> impl Strategy<Value = String> {
        string_regex(r"\t{1,4}").unwrap()
    }

    fn mixed_indent_strategy() -> impl Strategy<Value = String> {
        prop::collection::vec(prop_oneof![Just(' '), Just('\t')], 2..6)
            .prop_filter("requires space and tab", |chars| {
                chars.iter().any(|c| *c == ' ') && chars.iter().any(|c| *c == '\t')
            })
            .prop_map(|chars| chars.into_iter().collect())
    }

    fn newline_strategy() -> impl Strategy<Value = String> {
        prop_oneof![Just("\n".to_string()), Just("\r\n".to_string())]
    }

    fn task_id_strategy() -> impl Strategy<Value = String> {
        string_regex(r"[A-Z0-9-]{1,8}").unwrap()
    }

    fn safe_line_strategy() -> impl Strategy<Value = String> {
        string_regex(r"[A-Za-z0-9][A-Za-z0-9 .,]{0,20}").unwrap()
    }

    fn mixed_whitespace_line_strategy() -> impl Strategy<Value = String> {
        (
            whitespace_strategy(),
            safe_line_strategy(),
            whitespace_strategy(),
        )
            .prop_map(|(leading, content, trailing)| format!("{leading}{content}{trailing}"))
    }

    fn noise_line_strategy() -> impl Strategy<Value = String> {
        string_regex(r"@[A-Za-z0-9]{1,8}").unwrap()
    }

    fn malformed_checkbox_prefix_strategy() -> impl Strategy<Value = String> {
        (0usize..=2, 0usize..=2)
            .prop_filter("exclude valid spacing", |(outer, inner)| {
                !(*outer == 1 && *inner == 1)
            })
            .prop_map(|(outer, inner)| format!("-{}[{}]", " ".repeat(outer), " ".repeat(inner)))
    }

    fn empty_h2_heading_strategy() -> impl Strategy<Value = String> {
        (whitespace_strategy(), whitespace_strategy())
            .prop_map(|(leading, trailing)| format!("{leading}## {trailing}"))
    }

    fn tabbed_heading_near_miss_strategy() -> impl Strategy<Value = String> {
        (
            whitespace_strategy(),
            string_regex(r"[A-Za-z0-9][A-Za-z0-9 ]{0,12}").unwrap(),
            prop_oneof![
                Just("\t".to_string()),
                Just("\t ".to_string()),
                Just("\t\t".to_string())
            ],
        )
            .prop_map(|(leading, title, sep)| format!("{leading}##{sep}{title}"))
    }

    fn separator_near_miss_strategy() -> impl Strategy<Value = String> {
        (
            whitespace_strategy(),
            string_regex(r"[A-Za-z0-9-]{1,3}").unwrap(),
            whitespace_strategy(),
        )
            .prop_map(|(leading, suffix, trailing)| format!("{leading}---{suffix}{trailing}"))
    }

    #[test]
    fn task_blocks_from_contents_returns_empty_when_no_blocks_exist() {
        let contents = "## Overview\n- [ ] Not a task block\n---\n";
        let blocks = task_blocks_from_contents(contents);
        assert!(blocks.is_empty());
    }

    #[test]
    fn task_blocks_from_contents_ends_on_separator_and_section_heading() {
        let contents =
            "### Task A\n- [ ] First\n---\n### Task B\n- [ ] Second\n## Success Criteria\n- Done\n";
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
    fn task_blocks_from_contents_handles_adjacent_blocks_and_trailing_sections() {
        let contents =
            "### Task A\n- [ ] First\n### Task B\n- [ ] Second\n## Trailing\n- [ ] Not a task";
        let blocks = task_blocks_from_contents(contents);

        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].contains("### Task A"));
        assert!(!blocks[0].contains("### Task B"));
        assert!(blocks[1].contains("### Task B"));
        assert!(blocks[1].contains("- [ ] Second"));
        assert!(!blocks[1].contains("Trailing"));
    }

    #[test]
    fn task_blocks_from_contents_terminates_on_h2_heading_with_trailing_spaces() {
        let contents = "### Task A\n- [ ] First\n## Notes   \n- [ ] Outside\n---\n";
        let blocks = task_blocks_from_contents(contents);

        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].contains("### Task A"));
        assert!(blocks[0].contains("- [ ] First"));
        assert!(!blocks[0].contains("## Notes"));
        assert!(!blocks[0].contains("Outside"));
    }

    #[test]
    fn task_blocks_from_contents_ignores_tabbed_heading_near_miss() {
        let contents = "### Task A\n- [ ] First\n##\tNotes\n- [ ] Still inside\n---\n- [ ] Outside";
        let blocks = task_blocks_from_contents(contents);

        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].contains("##\tNotes"));
        assert!(blocks[0].contains("Still inside"));
        assert!(!blocks[0].contains("Outside"));
    }

    #[test]
    fn is_task_header_accepts_leading_whitespace() {
        assert!(is_task_header("  ### Task COV-13"));
    }

    #[test]
    fn is_task_header_accepts_trailing_space_without_id() {
        assert!(is_task_header("### Task "));
    }

    #[test]
    fn is_task_header_accepts_tab_leading_whitespace() {
        assert!(is_task_header("\t### Task COV-13"));
        assert!(is_task_header("\t \t### Task COV-13"));
    }

    #[test]
    fn is_unchecked_line_accepts_leading_whitespace() {
        assert!(is_unchecked_line("   - [ ] Edge case"));
    }

    #[test]
    fn is_unchecked_line_accepts_tab_leading_whitespace() {
        assert!(is_unchecked_line("\t- [ ] Edge case"));
    }

    #[test]
    fn is_unchecked_line_accepts_crlf_and_mixed_leading_whitespace() {
        assert!(is_unchecked_line("- [ ] Edge case\r"));
        assert!(is_unchecked_line(" \t- [ ] Edge case\r"));
    }

    #[test]
    fn is_unchecked_line_rejects_spacing_near_misses() {
        assert!(!is_unchecked_line("-  [ ] Edge case"));
        assert!(!is_unchecked_line("- [  ] Edge case"));
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
    fn is_task_block_end_accepts_tabbed_separator_and_heading() {
        assert!(is_task_block_end("\t---\t"));
        assert!(is_task_block_end("\t## Notes"));
    }

    #[test]
    fn is_task_block_end_accepts_crlf_lines() {
        assert!(is_task_block_end("---\r"));
        assert!(is_task_block_end("## Notes\r"));
    }

    #[test]
    fn is_task_block_end_rejects_empty_h2_headings() {
        assert!(!is_task_block_end("## "));
        assert!(!is_task_block_end("  ##    "));
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
    fn is_task_block_end_rejects_tabbed_heading_without_space() {
        assert!(!is_task_block_end("##\tNotes"));
        assert!(!is_task_block_end("\t##\tNotes"));
    }

    #[test]
    fn is_task_block_end_rejects_tabbed_heading_without_title() {
        assert!(!is_task_block_end("##\t"));
        assert!(!is_task_block_end(" \t##\t"));
    }

    #[test]
    fn is_task_block_end_rejects_spacing_near_misses() {
        assert!(!is_task_block_end("## \t"));
        assert!(!is_task_block_end("##\t Notes"));
        assert!(!is_task_block_end(" ---#"));
    }

    #[test]
    fn is_task_header_rejects_malformed_headings() {
        assert!(!is_task_header("###Task COV-29"));
        assert!(!is_task_header("## Task COV-29"));
        assert!(!is_task_header("#### Task COV-29"));
        assert!(!is_task_header("### Tasks COV-29"));
        assert!(!is_task_header("### Task"));
    }

    #[test]
    fn is_task_header_rejects_tabbed_headings() {
        assert!(!is_task_header("###\tTask COV-29"));
        assert!(!is_task_header("### Task\tCOV-29"));
    }

    #[test]
    fn is_task_header_rejects_spacing_near_misses() {
        assert!(!is_task_header("###  Task COV-29"));
        assert!(!is_task_header("### Task\t COV-29"));
    }

    #[test]
    fn is_unchecked_line_rejects_tabbed_near_misses() {
        assert!(!is_unchecked_line("-\t[ ] Edge case"));
        assert!(!is_unchecked_line("- [\t] Edge case"));
    }

    #[test]
    fn is_unchecked_line_rejects_spacing_near_misses_with_tabs() {
        assert!(!is_unchecked_line("- \t[ ] Edge case"));
        assert!(!is_unchecked_line("-\t [ ] Edge case"));
    }

    proptest! {
        #[test]
        fn prop_task_blocks_from_contents_terminates_on_separator(
            header_leading in whitespace_strategy(),
            separator_leading in whitespace_strategy(),
            separator_trailing in whitespace_strategy(),
            id in task_id_strategy(),
            body in prop::collection::vec(safe_line_strategy(), 0..4),
            suffix in prop::collection::vec(safe_line_strategy(), 0..3)
        ) {
            let header = format!("{}### Task {}", header_leading, id);
            let mut contents = header.clone();
            let mut expected = header;

            for line in body {
                contents.push('\n');
                contents.push_str(&line);
                expected.push('\n');
                expected.push_str(&line);
            }

            contents.push('\n');
            contents.push_str(&format!("{}---{}", separator_leading, separator_trailing));

            if !suffix.is_empty() {
                contents.push('\n');
                contents.push_str(&suffix.join("\n"));
            }

            let blocks_out = task_blocks_from_contents(&contents);
            prop_assert_eq!(blocks_out.len(), 1);
            prop_assert_eq!(&blocks_out[0], &expected);
            prop_assert!(!blocks_out[0].lines().any(|line| line.trim() == "---"));
        }

        #[test]
        fn prop_task_blocks_from_contents_terminates_on_separator_with_crlf(
            header_leading in whitespace_strategy(),
            separator_leading in whitespace_strategy(),
            separator_trailing in whitespace_strategy(),
            id in task_id_strategy(),
            body in prop::collection::vec(safe_line_strategy(), 0..4),
            suffix in prop::collection::vec(noise_line_strategy(), 0..3)
        ) {
            let header = format!("{}### Task {}", header_leading, id);
            let separator = format!("{}---{}", separator_leading, separator_trailing);
            let mut lines = Vec::new();
            lines.push(header.clone());
            let mut expected = header;

            for line in body {
                lines.push(line.clone());
                expected.push('\n');
                expected.push_str(&line);
            }

            lines.push(separator);
            lines.extend(suffix.iter().cloned());

            let contents = lines.join("\r\n");
            let blocks_out = task_blocks_from_contents(&contents);

            prop_assert_eq!(blocks_out.len(), 1);
            prop_assert_eq!(&blocks_out[0], &expected);
            prop_assert!(!blocks_out[0].lines().any(|line| line.trim() == "---"));
            for line in suffix.iter() {
                prop_assert!(!blocks_out[0].lines().any(|block_line| block_line == line));
            }
        }

        #[test]
        fn prop_task_blocks_from_contents_handles_tabbed_headers_and_separators_with_crlf(
            header_tabs in tab_prefix_strategy(),
            separator_tabs in tab_prefix_strategy(),
            separator_trailing in whitespace_strategy(),
            id in task_id_strategy(),
            body in prop::collection::vec(safe_line_strategy(), 0..4),
            suffix in prop::collection::vec(noise_line_strategy(), 0..3)
        ) {
            let header = format!("{}### Task {}", header_tabs, id);
            let separator = format!("{}---{}", separator_tabs, separator_trailing);
            let mut lines = Vec::new();
            lines.push(header.clone());
            let mut expected = header;

            for line in body {
                lines.push(line.clone());
                expected.push('\n');
                expected.push_str(&line);
            }

            lines.push(separator);
            lines.extend(suffix.iter().cloned());

            let contents = lines.join("\r\n");
            let blocks_out = task_blocks_from_contents(&contents);

            prop_assert_eq!(blocks_out.len(), 1);
            prop_assert_eq!(&blocks_out[0], &expected);
            prop_assert!(!blocks_out[0].lines().any(|line| line.trim() == "---"));
            for line in suffix.iter() {
                prop_assert!(!blocks_out[0].lines().any(|block_line| block_line == line));
            }
        }

        #[test]
        fn prop_task_blocks_from_contents_terminates_on_h2_heading(
            header_leading in whitespace_strategy(),
            heading_leading in whitespace_strategy(),
            id in task_id_strategy(),
            body in prop::collection::vec(safe_line_strategy(), 0..4),
            title in string_regex(r"[A-Za-z0-9][A-Za-z0-9 ]{0,12}").unwrap(),
            suffix in prop::collection::vec(safe_line_strategy(), 0..3)
        ) {
            let header = format!("{}### Task {}", header_leading, id);
            let heading = format!("{}## {}", heading_leading, title);
            let mut contents = header.clone();
            let mut expected = header;

            for line in body {
                contents.push('\n');
                contents.push_str(&line);
                expected.push('\n');
                expected.push_str(&line);
            }

            contents.push('\n');
            contents.push_str(&heading);

            if !suffix.is_empty() {
                contents.push('\n');
                contents.push_str(&suffix.join("\n"));
            }

            let blocks_out = task_blocks_from_contents(&contents);
            prop_assert_eq!(blocks_out.len(), 1);
            prop_assert_eq!(&blocks_out[0], &expected);
            prop_assert!(!blocks_out[0].lines().any(|line| line == heading));
        }

        #[test]
        fn prop_task_blocks_from_contents_terminates_on_h2_heading_with_crlf(
            header_leading in whitespace_strategy(),
            heading_leading in whitespace_strategy(),
            id in task_id_strategy(),
            body in prop::collection::vec(safe_line_strategy(), 0..4),
            title in string_regex(r"[A-Za-z0-9][A-Za-z0-9 ]{0,12}").unwrap(),
            suffix in prop::collection::vec(noise_line_strategy(), 0..3)
        ) {
            let header = format!("{}### Task {}", header_leading, id);
            let heading = format!("{}## {}", heading_leading, title);
            let mut lines = Vec::new();
            lines.push(header.clone());
            let mut expected = header;

            for line in body {
                lines.push(line.clone());
                expected.push('\n');
                expected.push_str(&line);
            }

            lines.push(heading.clone());
            lines.extend(suffix.iter().cloned());

            let contents = lines.join("\r\n");
            let blocks_out = task_blocks_from_contents(&contents);

            prop_assert_eq!(blocks_out.len(), 1);
            prop_assert_eq!(&blocks_out[0], &expected);
            prop_assert!(!blocks_out[0].lines().any(|line| line == heading));
            for line in suffix.iter() {
                prop_assert!(!blocks_out[0].lines().any(|block_line| block_line == line));
            }
        }

        #[test]
        fn prop_task_blocks_from_contents_terminates_on_new_task_header(
            header_leading in whitespace_strategy(),
            next_leading in whitespace_strategy(),
            first_id in task_id_strategy(),
            second_id in task_id_strategy(),
            first_body in prop::collection::vec(safe_line_strategy(), 0..3),
            second_body in prop::collection::vec(safe_line_strategy(), 0..3),
            suffix in prop::collection::vec(safe_line_strategy(), 0..2)
        ) {
            let first_header = format!("{}### Task {}", header_leading, first_id);
            let second_header = format!("{}### Task {}", next_leading, second_id);
            let mut contents = first_header.clone();
            let mut expected_first = first_header;

            for line in &first_body {
                contents.push('\n');
                contents.push_str(line);
                expected_first.push('\n');
                expected_first.push_str(line);
            }

            contents.push('\n');
            contents.push_str(&second_header);

            let mut expected_second = second_header;
            for line in &second_body {
                contents.push('\n');
                contents.push_str(line);
                expected_second.push('\n');
                expected_second.push_str(line);
            }

            if !suffix.is_empty() {
                contents.push('\n');
                contents.push_str(&suffix.join("\n"));
                expected_second.push('\n');
                expected_second.push_str(&suffix.join("\n"));
            }

            let blocks_out = task_blocks_from_contents(&contents);
            prop_assert_eq!(blocks_out.len(), 2);
            prop_assert_eq!(&blocks_out[0], &expected_first);
            prop_assert_eq!(&blocks_out[1], &expected_second);
        }

        #[test]
        fn prop_task_blocks_from_contents_ignores_empty_h2_heading(
            header_leading in whitespace_strategy(),
            id in task_id_strategy(),
            body in prop::collection::vec(safe_line_strategy(), 0..3),
            empty_heading in empty_h2_heading_strategy(),
            suffix in prop::collection::vec(safe_line_strategy(), 0..2)
        ) {
            let header = format!("{}### Task {}", header_leading, id);
            let mut contents = header.clone();
            let mut expected = header;

            for line in &body {
                contents.push('\n');
                contents.push_str(line);
                expected.push('\n');
                expected.push_str(line);
            }

            contents.push('\n');
            contents.push_str(&empty_heading);
            expected.push('\n');
            expected.push_str(&empty_heading);

            if !suffix.is_empty() {
                contents.push('\n');
                contents.push_str(&suffix.join("\n"));
                expected.push('\n');
                expected.push_str(&suffix.join("\n"));
            }

            let blocks_out = task_blocks_from_contents(&contents);
            prop_assert_eq!(blocks_out.len(), 1);
            prop_assert_eq!(&blocks_out[0], &expected);
        }

        #[test]
        fn prop_task_blocks_from_contents_ignores_tabbed_h2_near_miss_with_crlf(
            header_leading in whitespace_strategy(),
            id in task_id_strategy(),
            body in prop::collection::vec(safe_line_strategy(), 0..3),
            near_miss in tabbed_heading_near_miss_strategy(),
            suffix in prop::collection::vec(safe_line_strategy(), 1..3)
        ) {
            let header = format!("{}### Task {}", header_leading, id);
            let mut lines = Vec::new();
            lines.push(header.clone());
            let mut expected = header;

            for line in &body {
                lines.push(line.clone());
                expected.push('\n');
                expected.push_str(line);
            }

            lines.push(near_miss.clone());
            expected.push('\n');
            expected.push_str(&near_miss);

            lines.extend(suffix.iter().cloned());
            expected.push('\n');
            expected.push_str(&suffix.join("\n"));

            let contents = lines.join("\r\n");
            let blocks_out = task_blocks_from_contents(&contents);

            prop_assert_eq!(blocks_out.len(), 1);
            prop_assert_eq!(&blocks_out[0], &expected);
        }

        #[test]
        fn prop_task_blocks_from_contents_ignores_separator_near_miss(
            header_leading in whitespace_strategy(),
            id in task_id_strategy(),
            body in prop::collection::vec(safe_line_strategy(), 0..3),
            near_miss in separator_near_miss_strategy(),
            suffix in prop::collection::vec(safe_line_strategy(), 0..2)
        ) {
            let header = format!("{}### Task {}", header_leading, id);
            let mut contents = header.clone();
            let mut expected = header;

            for line in &body {
                contents.push('\n');
                contents.push_str(line);
                expected.push('\n');
                expected.push_str(line);
            }

            contents.push('\n');
            contents.push_str(&near_miss);
            expected.push('\n');
            expected.push_str(&near_miss);

            if !suffix.is_empty() {
                contents.push('\n');
                contents.push_str(&suffix.join("\n"));
                expected.push('\n');
                expected.push_str(&suffix.join("\n"));
            }

            let blocks_out = task_blocks_from_contents(&contents);
            prop_assert_eq!(blocks_out.len(), 1);
            prop_assert_eq!(&blocks_out[0], &expected);
        }

        #[test]
        fn prop_task_blocks_from_contents_ignores_separator_near_miss_with_crlf(
            header_leading in whitespace_strategy(),
            id in task_id_strategy(),
            body in prop::collection::vec(safe_line_strategy(), 0..3),
            near_miss in separator_near_miss_strategy(),
            suffix in prop::collection::vec(safe_line_strategy(), 0..2)
        ) {
            let header = format!("{}### Task {}", header_leading, id);
            let mut lines = Vec::new();
            lines.push(header.clone());
            let mut expected = header;

            for line in &body {
                lines.push(line.clone());
                expected.push('\n');
                expected.push_str(line);
            }

            lines.push(near_miss.clone());
            expected.push('\n');
            expected.push_str(&near_miss);

            if !suffix.is_empty() {
                lines.extend(suffix.iter().cloned());
                expected.push('\n');
                expected.push_str(&suffix.join("\n"));
            }

            let contents = lines.join("\r\n");
            let blocks_out = task_blocks_from_contents(&contents);
            prop_assert_eq!(blocks_out.len(), 1);
            prop_assert_eq!(&blocks_out[0], &expected);
        }

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
        fn prop_task_blocks_from_contents_excludes_stray_lines(
            prefix in prop::collection::vec(noise_line_strategy(), 0..3),
            body in prop::collection::vec(safe_line_strategy(), 0..4),
            suffix in prop::collection::vec(noise_line_strategy(), 0..3),
            end_with_separator in any::<bool>(),
            id in task_id_strategy()
        ) {
            let mut contents = String::new();
            if !prefix.is_empty() {
                contents.push_str(&prefix.join("\n"));
                contents.push('\n');
            }

            let header = format!("### Task {}", id);
            contents.push_str(&header);

            let mut expected = header;
            for line in &body {
                contents.push('\n');
                contents.push_str(line);
                expected.push('\n');
                expected.push_str(line);
            }

            contents.push('\n');
            if end_with_separator {
                contents.push_str("---");
            } else {
                contents.push_str("## End");
            }

            if !suffix.is_empty() {
                contents.push('\n');
                contents.push_str(&suffix.join("\n"));
            }

            let blocks_out = task_blocks_from_contents(&contents);
            prop_assert_eq!(blocks_out.len(), 1);

            let block = &blocks_out[0];
            prop_assert_eq!(block, &expected);
            for line in prefix.iter().chain(suffix.iter()) {
                prop_assert!(!block.contains(line));
            }
        }

        #[test]
        fn prop_task_blocks_from_contents_handles_crlf_and_mixed_whitespace(
            newline in newline_strategy(),
            header_leading in whitespace_strategy(),
            separator_leading in whitespace_strategy(),
            separator_trailing in whitespace_strategy(),
            id in task_id_strategy(),
            prefix in prop::collection::vec(noise_line_strategy(), 0..3),
            body in prop::collection::vec(mixed_whitespace_line_strategy(), 0..4),
            suffix in prop::collection::vec(noise_line_strategy(), 0..3)
        ) {
            let header = format!("{}### Task {}", header_leading, id);
            let separator = format!("{}---{}", separator_leading, separator_trailing);

            let mut lines = Vec::new();
            lines.extend(prefix.iter().cloned());
            lines.push(header.clone());

            let mut expected = header;
            for line in &body {
                lines.push(line.clone());
                expected.push('\n');
                expected.push_str(line);
            }

            lines.push(separator);
            lines.extend(suffix.iter().cloned());

            let contents = lines.join(&newline);
            let blocks_out = task_blocks_from_contents(&contents);

            prop_assert_eq!(blocks_out.len(), 1);
            let block = &blocks_out[0];
            prop_assert_eq!(block, &expected);
            prop_assert!(!block.contains('\r'));
            for line in prefix.iter().chain(suffix.iter()) {
                prop_assert!(!block.contains(line));
            }
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
        fn prop_is_task_block_end_accepts_tabbed_heading(
            leading in tab_prefix_strategy(),
            title in string_regex(r"[A-Za-z0-9][A-Za-z0-9 ]{0,12}").unwrap()
        ) {
            let line = format!("{}## {}", leading, title);
            prop_assert!(is_task_block_end(&line));
        }

        #[test]
        fn prop_is_task_block_end_rejects_tabbed_heading_near_misses(
            line in tabbed_heading_near_miss_strategy()
        ) {
            prop_assert!(!is_task_block_end(&line));
        }

        #[test]
        fn prop_is_task_block_end_rejects_whitespace_only_heading(
            leading in whitespace_strategy(),
            trailing in string_regex(r"[ \t]{0,6}").unwrap()
        ) {
            let line = format!("{leading}## {trailing}");
            prop_assert!(!is_task_block_end(&line));
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
            prop_assume!(!line.trim_start().starts_with("## "));
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
        fn prop_is_unchecked_line_accepts_mixed_whitespace_prefix(
            leading in mixed_indent_strategy(),
            tail in string_regex(r"[ \tA-Za-z0-9]{0,12}").unwrap()
        ) {
            let line = format!("{leading}- [ ]{tail}");
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

        #[test]
        fn prop_is_unchecked_line_rejects_spacing_variants(
            leading in whitespace_strategy(),
            prefix in malformed_checkbox_prefix_strategy(),
            tail in string_regex(r"[^\n]{0,12}").unwrap()
        ) {
            let line = format!("{}{}{}", leading, prefix, tail);
            prop_assert!(!is_unchecked_line(&line));
        }

        #[test]
        fn prop_is_unchecked_line_rejects_tabbed_near_miss_variants(
            leading in whitespace_strategy(),
            tail in string_regex(r"[^\n]{0,12}").unwrap(),
            variant in prop_oneof![
                Just("-\t[ ]".to_string()),
                Just("- [\t]".to_string()),
                Just("-\t [ ]".to_string()),
                Just("- [\t ]".to_string())
            ]
        ) {
            let line = format!("{}{}{}", leading, variant, tail);
            prop_assert!(!is_unchecked_line(&line));
        }
    }
}
