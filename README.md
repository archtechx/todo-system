# An intuitive system for organizing TODOs in code

## Why?
- In-code TODOs are often more useful than tasks in project management apps, as they're co-located with the relevant code
- In many projects, they grow out of control (everything is just a "TODO" with nothing indicating priority or category)
- Working on related tasks at the same time makes more sense than context switching between different tasks. However, proximity in code doesn't necessarily correlate to "task proximity". Two TODOs in the same file may be two completely different tasks, but two TODOs across different files may be very similar
- There's a difference between TODOs written as quick notes while context switching between different parts of the code *while working on a single task*, and long term TODOs that are part of the code

## Solution

Numbered (priority) todos, indicating what needs to be solved *now* and in what order.

And category todos grouping tasks into categories.

## Spec

### Priority todos

Syntax: `todo{number} {description?}`, number can be:
- `{1, 2, 3, ...}` — the higher the number the *lower* the priority (i.e. `todo1` is more urgent than `todo2`)
- `0`/`00`/`000`/... — the more zeroes, the more urgent the task is

As an example:
```
// todo000 something part of currently written code, needs to be solved first (highest amount of 0s)
// todo00 something that needs to be resolved right after ^ this task
// todo0 something that needs to be resolved after the above task
// todo1 something even lower priority
// todo2 something *even* lower priority
// todo3 etc
```

In general, all priority todos (todos with numbers) need to be solved before the current work is considered complete.

When making a lot of changes in a single commit, this may mean resolving all priority todos before committing changes.

In practice, the usage may look like this:
- you're working on `x`, and as part of that you need to work on `y`
- there are still some things unresolved in `x`, but you need to work on `y` to move forward. You leave a `todo1` in the `x` part of the code
- while working on `y`, you leave a `todo0` for something that needs to be resolved before returning to `x`
- while working on that, you notice another thing that needs to be solved, even before the `todo0`. You leave a `todo00`

### Category todos

Syntax: `todo@{category} {description?}`

A way to group TODOs by category.

Examples:
```
todo@responsivity Hide this on mobile
todo@darkmode Improve input styling
todo@types
todo@testing test this
```

### Generic todos

Any todos that don't fall into the two categories above (i.e. their syntax isn't `todo{number}` or `todo@{category}`).

```
TODO: Fix this
todo refactor
```

## Markdown files

In some larger projects, we also keep track of TODOs in markdown files. This is useful when the task is more abstract and not immediately related to any given piece of code.

We follow this convention:

`README.md`

```md
# Project name

## Some section

...

## TODOs

- foo
- bar
```

And in larger projects, we have often have a dedicated file for TODOs:

`todo.md`

```md
- Generic todo 1
- Generic todo 2

## Category 1
- foo
- bar

## Category 2
- abc
- def
```

## Validating code

As a general rule, in our code, priority todos **may not be pushed into master**. They need to be resolved before committing (ideally) or before merging PRs (when working on larger things).

To validate this automatically, you can set up a simple GitHub Action:

```yaml
on: [push, pull_request]

jobs:
  validate:
    name: Validate code
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v2
    - name: Check for todo0
      run: '! grep -r "todo0" --exclude-dir=workflows .'
      if: always()
    - name: Check for todo1
      run: '! grep -r "todo1" --exclude-dir=workflows .'
      if: always()
    - name: Check for todo2
      run: '! grep -r "todo2" --exclude-dir=workflows .'
      if: always()
```

## CLI tool

The benefit of TODOs in code is that they're searchable, and searching `todo` makes *any* kind of todo show up (since it doesn't matter whether it's followed by a number, an at sign, or whitespace).

That said, this repo includes a simple CLI tool written in Rust for getting an easy-to-read, ANSI-colored, Markdown-formatted list of all todos in a project.

Usage:
```
todos --exclude node_modules src/
```

Output:
```md
# TODOs

<!-- priority todos -->
## todo00
- [ ] foo (/file:123)
- [ ] bar (/file:456)

## todo0
- [ ] abc (/file:123)
- [ ] def (/file:456)

<!-- category todos -->
## testing
- [ ] abc (/file:123)
- [ ] def (/file:456)

## responsivity
- [ ] abc (/file:123)
- [ ] def (/file:456)

<!-- generic todos -->
## Other
- [ ] abc (/file:123)
- [ ] def (/file:456)
```

(without the HTML comments).

Notes:
- `node_modules/` (for npm) and `vendor/` (for composer) are excluded by default
- paths starting with `.` are **always** excluded
- `--exclude`s are relative to the current working directory, not passed paths (including default excludes mentioned above). If you're running the script for another folder and want to exclude folders there, type out the path in `--exclude`
- Passing any excludes overrides the default excludes, so if you want to *add* to the list of excludes, you need to re-define the default ones as well (e.g. `-e node_modules`)

The tool also scans a todo.md file (path can be provided using `--todos`):
- all TODOs have to be list items (`- foo` or `- [ ] foo`)
- any TODOs *above* the first heading are considered generic TODOs
- any TODOs under a heading are considered category TODOs, with the heading being the category name
- any TODOs with numbers are added to the list of priority TODOs

Scanning TODOs in a README.md file is also supported:
- all TODOs have to be list items (`- foo` or `- [ ] foo`)
- they have to be directly under a `TODO[s:]` (lower or uppercase) heading

See the `samples/` folder for examples.

To omit ANSI formatting and get raw markdown output, set `NO_COLOR=1` or `TERM=dumb`.

### Installation

There are no downloadable builds at the moment. To compile the tool manually:
1. Set up Rust locally https://www.rust-lang.org/tools/install
2. Clone the repo
3. `cargo build --release`
4. The binary (`todos`) will appear will `target/release`. Add it to your PATH or create a bash alias:
   ```sh
   alias todos="/path/to/todos"
   ```
