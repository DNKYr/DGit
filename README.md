# DGit

A Git re-implementation in Rust — a learning-focused clone of the core Git plumbing and porcelain commands.

## Features

DGit implements the full object model (blob, tree, commit, tag), index/staging area, references, and a growing set of porcelain commands. It is **cross-compatible** with real Git — repositories, objects, and commits created by DGit can be read by Git and vice versa.

## Commands

| Command | Status | Description |
|---|---|---|
| `init [path]` | Done | Initialize a new repository |
| `add <files...>` | Done | Stage file contents into the index |
| `rm [--cached] [-f] <files...>` | Done | Remove files from index and working tree |
| `commit -m <msg>` | Done | Record changes to the repository |
| `status` | Done | Show working tree status (staged, unstaged, untracked) |
| `ls-files [-s]` | Done | List files in the index |
| `write-tree` | Done | Create a tree object from the index |
| `cat-file <type> <object>` | Done | Read and print object contents |
| `hash-object [-w] <file>` | Done | Compute SHA1 of a file |
| `log [commit]` | Done | Display commit history (Graphviz DOT) |
| `ls-tree [-r] <tree>` | Done | List contents of a tree object |
| `checkout <commit> <path>` | Done | Checkout a commit into a directory |
| `show-ref` | Done | List all references |
| `rev-parse <name>` | Done | Resolve a revision to its SHA1 |
| `tag [-a] [name] [object]` | Done | List or create tags |

## Internals

- **Object store** — blob, tree, commit, tag objects with SHA1 hashing and zlib compression
- **Index (staging area)** — version 2 `.git/index` binary format with read/write and SHA1 checksum validation
- **References** — symbolic refs, branch resolution, ref listing, and ref writing
- **Config** — read `.git/config` for `user.name`, `user.email`, with subsection support
- **Gitignore** — pattern matching with `*`, `?`, `**`, `[a-z]` character classes, negation, and parent-directory checking

## Build

```sh
cargo build --release
```

## Usage

```sh
# Initialize a repository
dgit init my-project
cd my-project

# Set user info (or use GIT_AUTHOR_NAME / GIT_AUTHOR_EMAIL env vars)
git config user.name "Your Name"
git config user.email "you@example.com"

# Stage and commit
echo "Hello, world!" > README.md
dgit add README.md
dgit commit -m "initial commit"

# Check status
dgit status
```

DGit reads user identity from `.git/config` or the environment variables `GIT_AUTHOR_NAME` and `GIT_AUTHOR_EMAIL`. Timestamps use UTC (`+0000`).

## License

MIT
