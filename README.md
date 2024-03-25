# rs-email-parser

[![CI](https://github.com/philiprehberger/rs-email-parser/actions/workflows/ci.yml/badge.svg)](https://github.com/philiprehberger/rs-email-parser/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/philiprehberger-email-parser.svg)](https://crates.io/crates/philiprehberger-email-parser)
[![License](https://img.shields.io/github/license/philiprehberger/rs-email-parser)](LICENSE)

RFC 5322 compliant email address parsing, validation, and normalization

## Installation

```toml
[dependencies]
philiprehberger-email-parser = "0.1.0"
```

## Usage

```rust
use philiprehberger_email_parser::Email;

// Parse and validate
let email = Email::parse("user@example.com")?;
assert_eq!(email.local_part(), "user");
assert_eq!(email.domain(), "example.com");

// Quick validation
assert!(Email::is_valid("user@example.com"));
assert!(!Email::is_valid("not-an-email"));

// Display name
let email = Email::parse("\"John Doe\" <john@example.com>")?;
assert_eq!(email.display_name(), Some("John Doe"));
```

### Normalization

```rust
let email = Email::parse("User+tag@Example.COM")?;
let normalized = email.normalize();
assert_eq!(normalized.domain(), "example.com");

let clean = email.without_plus_alias();
assert_eq!(clean.local_part(), "User");
```

### Role address detection

```rust
let email = Email::parse("admin@example.com")?;
assert!(email.is_role_address());
```

## API

| Function / Type | Description |
|----------------|-------------|
| `Email::parse(input)` | Parse and validate an email address |
| `Email::is_valid(input)` | Quick boolean validation |
| `.local_part()` | Get the local part |
| `.domain()` | Get the domain |
| `.display_name()` | Get the display name (if any) |
| `.normalize()` | Lowercase domain |
| `.without_plus_alias()` | Remove + alias |
| `.is_role_address()` | Check if it's a role address |

## Development

```bash
cargo test
cargo clippy -- -D warnings
```

## License

MIT
