# Changelog

## 0.3.0 (2026-04-28)

- Add `is_disposable_provider()` method recognizing 12 throwaway email providers
- Add `is_corporate()` convenience returning `!is_free_provider() && !is_disposable_provider()`
- Add `to_canonical()` combining `normalize()` and `without_plus_alias()` for dedup keys
- Add `tld()` returning the last domain label (or `None` for IP literals)

## 0.2.0 (2026-04-05)

- Add `is_free_provider()` method to check if domain belongs to a known free email provider
- Recognized providers: gmail.com, yahoo.com, hotmail.com, outlook.com, aol.com, protonmail.com, icloud.com, mail.com, zoho.com

## 0.1.3 (2026-03-31)

- Standardize README to 3-badge format with emoji Support section
- Update CI checkout action to v5 for Node.js 24 compatibility

## 0.1.2 (2026-03-27)

- Add GitHub issue templates, PR template, and dependabot configuration
- Update README badges and add Support section

## 0.1.1 (2026-03-20)

- Re-release with registry token configured

## 0.1.0 (2026-03-19)

- Initial release
- RFC 5322 compliant email address parsing
- Structured access to local part, domain, and display name
- Email normalization (lowercase domain, handle + aliases)
- Role address detection
- Display and FromStr trait implementations
- Optional serde support
