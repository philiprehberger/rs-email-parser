//! RFC 5322 compliant email address parsing, validation, and normalization.
//!
//! This crate provides an [`Email`] type for parsing, validating, and manipulating
//! email addresses. It supports display names, quoted local parts, normalization,
//! plus-alias removal, and role address detection.
//!
//! # Examples
//!
//! ```
//! use philiprehberger_email_parser::Email;
//!
//! let email = Email::parse("user@example.com").unwrap();
//! assert_eq!(email.local_part(), "user");
//! assert_eq!(email.domain(), "example.com");
//!
//! // Quick validation
//! assert!(Email::is_valid("user@example.com"));
//! assert!(!Email::is_valid("invalid"));
//! ```

use std::fmt;
use std::str::FromStr;

/// Known free email provider domains.
const FREE_PROVIDERS: &[&str] = &[
    "gmail.com",
    "yahoo.com",
    "hotmail.com",
    "outlook.com",
    "aol.com",
    "protonmail.com",
    "icloud.com",
    "mail.com",
    "zoho.com",
];

/// Known disposable / throwaway email provider domains.
const DISPOSABLE_PROVIDERS: &[&str] = &[
    "mailinator.com",
    "tempmail.com",
    "10minutemail.com",
    "guerrillamail.com",
    "yopmail.com",
    "getnada.com",
    "throwawaymail.com",
    "dispostable.com",
    "fakeinbox.com",
    "sharklasers.com",
    "trashmail.com",
    "maildrop.cc",
];

/// Known role address local parts.
const ROLE_ADDRESSES: &[&str] = &[
    "admin",
    "info",
    "support",
    "sales",
    "contact",
    "noreply",
    "no-reply",
    "webmaster",
    "postmaster",
    "hostmaster",
    "abuse",
    "security",
    "billing",
    "help",
    "office",
    "team",
    "hello",
    "press",
    "media",
    "jobs",
    "careers",
    "legal",
    "compliance",
    "privacy",
    "mailer-daemon",
    "newsletter",
];

/// Errors that can occur when parsing an email address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmailError {
    /// The input string is empty.
    Empty,
    /// No `@` sign found in the address.
    MissingAtSign,
    /// Multiple `@` signs found outside of a quoted local part.
    MultipleAtSigns,
    /// The local part (before `@`) is empty.
    EmptyLocalPart,
    /// The domain (after `@`) is empty.
    EmptyDomain,
    /// The local part exceeds 64 characters.
    LocalPartTooLong,
    /// The domain exceeds 255 characters.
    DomainTooLong,
    /// The total address exceeds 254 characters.
    TotalTooLong,
    /// The local part contains invalid characters or formatting.
    InvalidLocalPart(String),
    /// The domain contains invalid characters or formatting.
    InvalidDomain(String),
}

impl fmt::Display for EmailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmailError::Empty => write!(f, "email address is empty"),
            EmailError::MissingAtSign => write!(f, "missing @ sign"),
            EmailError::MultipleAtSigns => write!(f, "multiple @ signs"),
            EmailError::EmptyLocalPart => write!(f, "empty local part"),
            EmailError::EmptyDomain => write!(f, "empty domain"),
            EmailError::LocalPartTooLong => write!(f, "local part exceeds 64 characters"),
            EmailError::DomainTooLong => write!(f, "domain exceeds 255 characters"),
            EmailError::TotalTooLong => write!(f, "total address exceeds 254 characters"),
            EmailError::InvalidLocalPart(reason) => {
                write!(f, "invalid local part: {}", reason)
            }
            EmailError::InvalidDomain(reason) => write!(f, "invalid domain: {}", reason),
        }
    }
}

impl std::error::Error for EmailError {}

/// A parsed and validated email address.
///
/// Supports standard email addresses, display names, quoted local parts,
/// and provides normalization and inspection utilities.
///
/// # Examples
///
/// ```
/// use philiprehberger_email_parser::Email;
///
/// let email = Email::parse("\"John Doe\" <john@example.com>").unwrap();
/// assert_eq!(email.display_name(), Some("John Doe"));
/// assert_eq!(email.local_part(), "john");
/// assert_eq!(email.domain(), "example.com");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String", into = "String"))]
pub struct Email {
    local_part: String,
    domain: String,
    display_name: Option<String>,
}

impl Email {
    /// Parse and validate an email address string.
    ///
    /// Accepts the following formats:
    /// - `user@example.com` (basic)
    /// - `"John Doe" <user@example.com>` (quoted display name)
    /// - `John Doe <user@example.com>` (unquoted display name)
    /// - `"user name"@example.com` (quoted local part)
    ///
    /// # Errors
    ///
    /// Returns an [`EmailError`] if the input is not a valid email address.
    pub fn parse(input: &str) -> Result<Email, EmailError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(EmailError::Empty);
        }

        let (display_name, address) = extract_display_name(input)?;
        let address = address.trim();

        if address.is_empty() {
            return Err(EmailError::Empty);
        }

        let (local_part, domain) = split_address(address)?;

        if local_part.is_empty() {
            return Err(EmailError::EmptyLocalPart);
        }
        if domain.is_empty() {
            return Err(EmailError::EmptyDomain);
        }

        if local_part.len() > 64 {
            return Err(EmailError::LocalPartTooLong);
        }
        if domain.len() > 255 {
            return Err(EmailError::DomainTooLong);
        }

        let total_len = local_part.len() + 1 + domain.len();
        if total_len > 254 {
            return Err(EmailError::TotalTooLong);
        }

        validate_local_part(&local_part)?;
        validate_domain(&domain)?;

        Ok(Email {
            local_part,
            domain,
            display_name,
        })
    }

    /// Quick boolean check whether a string is a valid email address.
    ///
    /// This is equivalent to `Email::parse(input).is_ok()`.
    pub fn is_valid(input: &str) -> bool {
        Email::parse(input).is_ok()
    }

    /// Returns the local part of the email address (before the `@`).
    pub fn local_part(&self) -> &str {
        &self.local_part
    }

    /// Returns the domain of the email address (after the `@`).
    pub fn domain(&self) -> &str {
        &self.domain
    }

    /// Returns the display name, if one was provided during parsing.
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    /// Returns the email address in `local@domain` form (without display name).
    pub fn as_str(&self) -> String {
        format!("{}@{}", self.local_part, self.domain)
    }

    /// Returns a new [`Email`] with the domain lowercased.
    ///
    /// The local part is preserved as-is since it is case-sensitive per RFC 5321.
    pub fn normalize(&self) -> Email {
        Email {
            local_part: self.local_part.clone(),
            domain: self.domain.to_lowercase(),
            display_name: self.display_name.clone(),
        }
    }

    /// Returns a new [`Email`] with everything after `+` in the local part removed.
    ///
    /// For example, `user+tag@example.com` becomes `user@example.com`.
    /// If there is no `+` in the local part, returns a clone.
    pub fn without_plus_alias(&self) -> Email {
        let local = if let Some(idx) = self.local_part.find('+') {
            self.local_part[..idx].to_string()
        } else {
            self.local_part.clone()
        };

        Email {
            local_part: local,
            domain: self.domain.clone(),
            display_name: self.display_name.clone(),
        }
    }

    /// Returns `true` if the local part matches a known role address.
    ///
    /// Role addresses include: admin, info, support, sales, contact, noreply,
    /// no-reply, webmaster, postmaster, hostmaster, abuse, security, billing,
    /// help, office, team, hello, press, media, jobs, careers, legal,
    /// compliance, privacy, mailer-daemon, newsletter.
    pub fn is_role_address(&self) -> bool {
        let lower = self.local_part.to_lowercase();
        ROLE_ADDRESSES.contains(&lower.as_str())
    }

    /// Returns `true` if the domain belongs to a known free email provider.
    ///
    /// Recognized providers: gmail.com, yahoo.com, hotmail.com, outlook.com,
    /// aol.com, protonmail.com, icloud.com, mail.com, zoho.com.
    ///
    /// The check is case-insensitive.
    pub fn is_free_provider(&self) -> bool {
        FREE_PROVIDERS.contains(&self.domain.to_lowercase().as_str())
    }

    /// Returns `true` if the domain belongs to a known disposable / throwaway
    /// email provider.
    ///
    /// Recognized providers include: mailinator.com, tempmail.com,
    /// 10minutemail.com, guerrillamail.com, yopmail.com, getnada.com,
    /// throwawaymail.com, dispostable.com, fakeinbox.com, sharklasers.com,
    /// trashmail.com, maildrop.cc.
    ///
    /// The check is case-insensitive.
    pub fn is_disposable_provider(&self) -> bool {
        DISPOSABLE_PROVIDERS.contains(&self.domain.to_lowercase().as_str())
    }

    /// Returns `true` if the domain is neither a free nor a disposable provider.
    ///
    /// Useful when filtering out personal addresses to focus on business / work
    /// emails. Note: this is a heuristic; a non-listed domain could still be a
    /// personal vanity address.
    pub fn is_corporate(&self) -> bool {
        !self.is_free_provider() && !self.is_disposable_provider()
    }

    /// Returns a canonicalized [`Email`] suitable for deduplication keys:
    /// domain lowercased and `+` alias removed from the local part.
    ///
    /// Equivalent to calling [`Self::normalize`] followed by
    /// [`Self::without_plus_alias`].
    pub fn to_canonical(&self) -> Email {
        self.normalize().without_plus_alias()
    }

    /// Returns the top-level domain label (the last segment of the domain).
    ///
    /// Returns `None` if the domain is an IP literal (e.g. `[192.168.1.1]`).
    pub fn tld(&self) -> Option<&str> {
        if self.domain.starts_with('[') {
            return None;
        }
        self.domain.rsplit('.').next()
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.display_name {
            Some(name) => write!(f, "\"{}\" <{}@{}>", name, self.local_part, self.domain),
            None => write!(f, "{}@{}", self.local_part, self.domain),
        }
    }
}

impl FromStr for Email {
    type Err = EmailError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Email::parse(s)
    }
}

#[cfg(feature = "serde")]
impl TryFrom<String> for Email {
    type Error = EmailError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Email::parse(&s)
    }
}

#[cfg(feature = "serde")]
impl From<Email> for String {
    fn from(email: Email) -> String {
        email.to_string()
    }
}

/// Extract an optional display name and the raw address from the input.
fn extract_display_name(input: &str) -> Result<(Option<String>, String), EmailError> {
    // Check for angle bracket format: ... <addr>
    if let Some(angle_start) = input.rfind('<') {
        if let Some(angle_end) = input.rfind('>') {
            if angle_end > angle_start {
                let address = input[angle_start + 1..angle_end].trim().to_string();
                let name_part = input[..angle_start].trim();

                let display_name = if name_part.is_empty() {
                    None
                } else {
                    // Strip surrounding quotes if present
                    let name = if name_part.starts_with('"') && name_part.ends_with('"') {
                        name_part[1..name_part.len() - 1].to_string()
                    } else {
                        name_part.to_string()
                    };
                    Some(name)
                };

                return Ok((display_name, address));
            }
        }
    }

    Ok((None, input.to_string()))
}

/// Split an address into local part and domain, handling quoted local parts.
fn split_address(address: &str) -> Result<(String, String), EmailError> {
    if let Some(after_quote) = address.strip_prefix('"') {
        // Quoted local part: find the closing quote
        if let Some(end_quote) = after_quote.find('"') {
            let local = after_quote[..end_quote].to_string();
            let rest = &after_quote[end_quote + 1..];
            if let Some(domain_str) = rest.strip_prefix('@') {
                let domain = domain_str.to_string();
                return Ok((local, domain));
            } else {
                return Err(EmailError::MissingAtSign);
            }
        } else {
            return Err(EmailError::InvalidLocalPart(
                "unclosed quote in local part".to_string(),
            ));
        }
    }

    // Non-quoted: split on @
    let at_count = address.chars().filter(|&c| c == '@').count();
    if at_count == 0 {
        return Err(EmailError::MissingAtSign);
    }
    if at_count > 1 {
        return Err(EmailError::MultipleAtSigns);
    }

    let at_pos = address.find('@').unwrap();
    let local = address[..at_pos].to_string();
    let domain = address[at_pos + 1..].to_string();

    Ok((local, domain))
}

/// Validate the local part of an email address.
fn validate_local_part(local: &str) -> Result<(), EmailError> {
    // Quoted local parts allow most characters
    // We already extracted the content from quotes in split_address,
    // so if we get here with a quoted-looking string that's fine.
    // For unquoted local parts, enforce strict rules.

    if local.starts_with('.') {
        return Err(EmailError::InvalidLocalPart(
            "cannot start with a dot".to_string(),
        ));
    }
    if local.ends_with('.') {
        return Err(EmailError::InvalidLocalPart(
            "cannot end with a dot".to_string(),
        ));
    }
    if local.contains("..") {
        return Err(EmailError::InvalidLocalPart(
            "consecutive dots not allowed".to_string(),
        ));
    }

    for ch in local.chars() {
        if !ch.is_alphanumeric()
            && ch != '.'
            && ch != '_'
            && ch != '+'
            && ch != '-'
            && ch != ' '
        {
            return Err(EmailError::InvalidLocalPart(format!(
                "invalid character '{}'",
                ch
            )));
        }
    }

    Ok(())
}

/// Validate the domain part of an email address.
fn validate_domain(domain: &str) -> Result<(), EmailError> {
    // IP address literal: [x.x.x.x]
    if domain.starts_with('[') && domain.ends_with(']') {
        let ip = &domain[1..domain.len() - 1];
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() == 4 {
            for part in &parts {
                if part.parse::<u8>().is_err() {
                    return Err(EmailError::InvalidDomain(
                        "invalid IP address literal".to_string(),
                    ));
                }
            }
            return Ok(());
        }
        return Err(EmailError::InvalidDomain(
            "invalid IP address literal".to_string(),
        ));
    }

    let labels: Vec<&str> = domain.split('.').collect();

    if labels.len() < 2 {
        return Err(EmailError::InvalidDomain(
            "must have at least two labels".to_string(),
        ));
    }

    for label in &labels {
        if label.is_empty() {
            return Err(EmailError::InvalidDomain("empty label".to_string()));
        }
        if label.len() > 63 {
            return Err(EmailError::InvalidDomain(
                "label exceeds 63 characters".to_string(),
            ));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(EmailError::InvalidDomain(
                "label cannot start or end with a hyphen".to_string(),
            ));
        }
        for ch in label.chars() {
            if !ch.is_alphanumeric() && ch != '-' {
                return Err(EmailError::InvalidDomain(format!(
                    "invalid character '{}' in label",
                    ch
                )));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_valid_emails() {
        let email = Email::parse("user@example.com").unwrap();
        assert_eq!(email.local_part(), "user");
        assert_eq!(email.domain(), "example.com");
        assert_eq!(email.display_name(), None);
    }

    #[test]
    fn test_dotted_local_part() {
        let email = Email::parse("user.name@example.com").unwrap();
        assert_eq!(email.local_part(), "user.name");
    }

    #[test]
    fn test_plus_tag() {
        let email = Email::parse("user+tag@example.com").unwrap();
        assert_eq!(email.local_part(), "user+tag");
    }

    #[test]
    fn test_display_name_quoted() {
        let email = Email::parse("\"John Doe\" <user@example.com>").unwrap();
        assert_eq!(email.display_name(), Some("John Doe"));
        assert_eq!(email.local_part(), "user");
        assert_eq!(email.domain(), "example.com");
    }

    #[test]
    fn test_display_name_unquoted() {
        let email = Email::parse("John Doe <user@example.com>").unwrap();
        assert_eq!(email.display_name(), Some("John Doe"));
        assert_eq!(email.local_part(), "user");
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(Email::parse(""), Err(EmailError::Empty));
    }

    #[test]
    fn test_just_at_sign() {
        let result = Email::parse("@");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_domain() {
        let result = Email::parse("user@");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_local_part() {
        let result = Email::parse("@domain.com");
        assert_eq!(result, Err(EmailError::EmptyLocalPart));
    }

    #[test]
    fn test_multiple_at_signs() {
        assert_eq!(
            Email::parse("user@@domain.com"),
            Err(EmailError::MultipleAtSigns)
        );
    }

    #[test]
    fn test_domain_starting_with_dot() {
        let result = Email::parse("user@.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_single_label_domain() {
        let result = Email::parse("user@domain");
        assert!(result.is_err());
    }

    #[test]
    fn test_local_part_starts_with_dot() {
        let result = Email::parse(".user@domain.com");
        assert!(matches!(result, Err(EmailError::InvalidLocalPart(_))));
    }

    #[test]
    fn test_consecutive_dots_in_local() {
        let result = Email::parse("user..name@domain.com");
        assert!(matches!(result, Err(EmailError::InvalidLocalPart(_))));
    }

    #[test]
    fn test_local_part_too_long() {
        let local = "a".repeat(65);
        let addr = format!("{}@example.com", local);
        assert_eq!(Email::parse(&addr), Err(EmailError::LocalPartTooLong));
    }

    #[test]
    fn test_domain_too_long() {
        let label = "a".repeat(63);
        // Build a domain with enough labels to exceed 255 chars
        let domain = format!("{}.{}.{}.{}.com", label, label, label, label);
        let addr = format!("u@{}", domain);
        assert_eq!(Email::parse(&addr), Err(EmailError::DomainTooLong));
    }

    #[test]
    fn test_total_too_long() {
        let local = "a".repeat(64);
        let domain_label = "b".repeat(63);
        let domain = format!("{}.{}.{}.com", domain_label, domain_label, domain_label);
        let addr = format!("{}@{}", local, domain);
        assert_eq!(Email::parse(&addr), Err(EmailError::TotalTooLong));
    }

    #[test]
    fn test_normalize() {
        let email = Email::parse("User@Example.COM").unwrap();
        let normalized = email.normalize();
        assert_eq!(normalized.local_part(), "User");
        assert_eq!(normalized.domain(), "example.com");
    }

    #[test]
    fn test_without_plus_alias() {
        let email = Email::parse("user+tag@example.com").unwrap();
        let clean = email.without_plus_alias();
        assert_eq!(clean.local_part(), "user");
        assert_eq!(clean.domain(), "example.com");
    }

    #[test]
    fn test_without_plus_alias_no_plus() {
        let email = Email::parse("user@example.com").unwrap();
        let clean = email.without_plus_alias();
        assert_eq!(clean.local_part(), "user");
    }

    #[test]
    fn test_is_role_address_true() {
        let email = Email::parse("admin@example.com").unwrap();
        assert!(email.is_role_address());

        let email = Email::parse("support@example.com").unwrap();
        assert!(email.is_role_address());

        let email = Email::parse("noreply@example.com").unwrap();
        assert!(email.is_role_address());

        let email = Email::parse("no-reply@example.com").unwrap();
        assert!(email.is_role_address());
    }

    #[test]
    fn test_is_role_address_false() {
        let email = Email::parse("john@example.com").unwrap();
        assert!(!email.is_role_address());
    }

    #[test]
    fn test_is_role_address_case_insensitive() {
        let email = Email::parse("Admin@example.com").unwrap();
        assert!(email.is_role_address());
    }

    #[test]
    fn test_is_valid_true() {
        assert!(Email::is_valid("user@example.com"));
        assert!(Email::is_valid("user.name@example.com"));
    }

    #[test]
    fn test_is_valid_false() {
        assert!(!Email::is_valid(""));
        assert!(!Email::is_valid("not-an-email"));
        assert!(!Email::is_valid("@"));
        assert!(!Email::is_valid("user@"));
    }

    #[test]
    fn test_display_without_name() {
        let email = Email::parse("user@example.com").unwrap();
        assert_eq!(email.to_string(), "user@example.com");
    }

    #[test]
    fn test_display_with_name() {
        let email = Email::parse("\"John Doe\" <user@example.com>").unwrap();
        assert_eq!(email.to_string(), "\"John Doe\" <user@example.com>");
    }

    #[test]
    fn test_display_roundtrip() {
        let original = Email::parse("\"John Doe\" <user@example.com>").unwrap();
        let displayed = original.to_string();
        let reparsed = Email::parse(&displayed).unwrap();
        assert_eq!(original.local_part(), reparsed.local_part());
        assert_eq!(original.domain(), reparsed.domain());
        assert_eq!(original.display_name(), reparsed.display_name());
    }

    #[test]
    fn test_display_roundtrip_basic() {
        let original = Email::parse("user@example.com").unwrap();
        let displayed = original.to_string();
        let reparsed = Email::parse(&displayed).unwrap();
        assert_eq!(original, reparsed);
    }

    #[test]
    fn test_from_str() {
        let email: Email = "user@example.com".parse().unwrap();
        assert_eq!(email.local_part(), "user");
        assert_eq!(email.domain(), "example.com");
    }

    #[test]
    fn test_from_str_invalid() {
        let result: Result<Email, _> = "not-an-email".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_quoted_local_part() {
        let email = Email::parse("\"user name\"@example.com").unwrap();
        assert_eq!(email.local_part(), "user name");
        assert_eq!(email.domain(), "example.com");
    }

    #[test]
    fn test_as_str() {
        let email = Email::parse("\"John\" <user@example.com>").unwrap();
        assert_eq!(email.as_str(), "user@example.com");
    }

    #[test]
    fn test_single_char_local() {
        let email = Email::parse("a@example.com").unwrap();
        assert_eq!(email.local_part(), "a");
    }

    #[test]
    fn test_single_char_labels() {
        let email = Email::parse("a@b.co").unwrap();
        assert_eq!(email.domain(), "b.co");
    }

    #[test]
    fn test_hyphen_in_domain() {
        let email = Email::parse("user@my-domain.com").unwrap();
        assert_eq!(email.domain(), "my-domain.com");
    }

    #[test]
    fn test_domain_label_leading_hyphen() {
        let result = Email::parse("user@-domain.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_domain_label_trailing_hyphen() {
        let result = Email::parse("user@domain-.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_ip_literal_domain() {
        let email = Email::parse("user@[192.168.1.1]").unwrap();
        assert_eq!(email.domain(), "[192.168.1.1]");
    }

    #[test]
    fn test_underscore_in_local() {
        let email = Email::parse("user_name@example.com").unwrap();
        assert_eq!(email.local_part(), "user_name");
    }

    #[test]
    fn test_hyphen_in_local() {
        let email = Email::parse("user-name@example.com").unwrap();
        assert_eq!(email.local_part(), "user-name");
    }

    #[test]
    fn test_eq_and_hash() {
        let a = Email::parse("user@example.com").unwrap();
        let b = Email::parse("user@example.com").unwrap();
        assert_eq!(a, b);

        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }

    #[test]
    fn test_error_display() {
        assert_eq!(EmailError::Empty.to_string(), "email address is empty");
        assert_eq!(EmailError::MissingAtSign.to_string(), "missing @ sign");
        assert_eq!(
            EmailError::LocalPartTooLong.to_string(),
            "local part exceeds 64 characters"
        );
    }

    #[test]
    fn test_angle_brackets_no_display_name() {
        let email = Email::parse("<user@example.com>").unwrap();
        assert_eq!(email.local_part(), "user");
        assert_eq!(email.display_name(), None);
    }

    #[test]
    fn test_is_free_provider_true() {
        let email = Email::parse("user@gmail.com").unwrap();
        assert!(email.is_free_provider());

        let email = Email::parse("user@yahoo.com").unwrap();
        assert!(email.is_free_provider());

        let email = Email::parse("user@hotmail.com").unwrap();
        assert!(email.is_free_provider());

        let email = Email::parse("user@outlook.com").unwrap();
        assert!(email.is_free_provider());

        let email = Email::parse("user@protonmail.com").unwrap();
        assert!(email.is_free_provider());

        let email = Email::parse("user@icloud.com").unwrap();
        assert!(email.is_free_provider());

        let email = Email::parse("user@aol.com").unwrap();
        assert!(email.is_free_provider());

        let email = Email::parse("user@mail.com").unwrap();
        assert!(email.is_free_provider());

        let email = Email::parse("user@zoho.com").unwrap();
        assert!(email.is_free_provider());
    }

    #[test]
    fn test_is_free_provider_false() {
        let email = Email::parse("user@example.com").unwrap();
        assert!(!email.is_free_provider());

        let email = Email::parse("user@company.org").unwrap();
        assert!(!email.is_free_provider());
    }

    #[test]
    fn test_is_free_provider_case_insensitive() {
        let email = Email::parse("user@Gmail.COM").unwrap();
        assert!(email.is_free_provider());

        let email = Email::parse("user@YAHOO.COM").unwrap();
        assert!(email.is_free_provider());
    }

    #[test]
    fn test_is_disposable_provider_true() {
        let email = Email::parse("user@mailinator.com").unwrap();
        assert!(email.is_disposable_provider());

        let email = Email::parse("user@10minutemail.com").unwrap();
        assert!(email.is_disposable_provider());

        let email = Email::parse("user@guerrillamail.com").unwrap();
        assert!(email.is_disposable_provider());
    }

    #[test]
    fn test_is_disposable_provider_false() {
        let email = Email::parse("user@gmail.com").unwrap();
        assert!(!email.is_disposable_provider());

        let email = Email::parse("user@example.com").unwrap();
        assert!(!email.is_disposable_provider());
    }

    #[test]
    fn test_is_disposable_provider_case_insensitive() {
        let email = Email::parse("user@MAILINATOR.COM").unwrap();
        assert!(email.is_disposable_provider());
    }

    #[test]
    fn test_is_corporate_true() {
        let email = Email::parse("user@example.com").unwrap();
        assert!(email.is_corporate());

        let email = Email::parse("user@company.org").unwrap();
        assert!(email.is_corporate());
    }

    #[test]
    fn test_is_corporate_false_for_free() {
        let email = Email::parse("user@gmail.com").unwrap();
        assert!(!email.is_corporate());
    }

    #[test]
    fn test_is_corporate_false_for_disposable() {
        let email = Email::parse("user@mailinator.com").unwrap();
        assert!(!email.is_corporate());
    }

    #[test]
    fn test_to_canonical() {
        let email = Email::parse("User+tag@Example.COM").unwrap();
        let canonical = email.to_canonical();
        assert_eq!(canonical.local_part(), "User");
        assert_eq!(canonical.domain(), "example.com");
    }

    #[test]
    fn test_to_canonical_no_plus() {
        let email = Email::parse("user@Example.COM").unwrap();
        let canonical = email.to_canonical();
        assert_eq!(canonical.local_part(), "user");
        assert_eq!(canonical.domain(), "example.com");
    }

    #[test]
    fn test_tld() {
        let email = Email::parse("user@example.com").unwrap();
        assert_eq!(email.tld(), Some("com"));

        let email = Email::parse("user@sub.example.co.uk").unwrap();
        assert_eq!(email.tld(), Some("uk"));
    }

    #[test]
    fn test_tld_ip_literal() {
        let email = Email::parse("user@[192.168.1.1]").unwrap();
        assert_eq!(email.tld(), None);
    }
}
