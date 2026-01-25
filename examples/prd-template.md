# [Project Name] - Product Requirements Document

## Overview

Brief description of the project and its purpose.

## Goals

- Primary goal 1
- Primary goal 2
- Primary goal 3

---

## Implementation Tasks

### Phase 1: Foundation

- [ ] Task 1: Description of first task
- [ ] Task 2: Description of second task
- [ ] Task 3: Description of third task

### Phase 2: Core Features

- [ ] Task 4: Description of fourth task
- [ ] Task 5: Description of fifth task
- [ ] Task 6: Description of sixth task

### Phase 3: Polish & Testing

- [ ] Task 7: Description of seventh task
- [ ] Task 8: Write tests
- [ ] Task 9: Update documentation

---

## Usage with gralph

```bash
# Start the autonomous loop
gralph start . --task-file PRD.md --max-iterations 30

# Monitor progress
gralph status

# View logs
gralph logs [project-name] --follow
```

## Notes

- Each task should be atomic and completable in a single Claude session
- Tasks are completed one at a time, marked with `- [x]` when done
- The loop continues until all `- [ ]` are converted to `- [x]`
