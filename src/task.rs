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
}
