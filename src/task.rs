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
