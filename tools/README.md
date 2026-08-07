# CLI Tools

A command-line tool for validating, formatting, and maintaining this repository.

# Development Setup

Configure `rust-analyzer` to use it from repository root dir. 

```jsonc
// .vscode/settings.json
{
    "rust-analyzer.linkedProjects": [
        "tools/Cargo.toml"
    ]
}
```

### Running the CLI tools

```sh
cd tools
cargo run -- --help
cargo run .. --fmt
```

#### Crawl Websites & Extract Job Posting

```sh
# Get API KEY from: https://aistudio.google.com/api-keys
export GEMINI_API_KEY = "..."
cargo run .. index
# or
cargo run .. index --log-file -c 2 --model <LLM_MODEl>
```
