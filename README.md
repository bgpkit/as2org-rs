# `as2org-rs`: utility crate for accessing CAIDA AS to organization mapping

*This readme is generated from the library's doc comments using [cargo-readme](https://github.com/livioribeiro/cargo-readme). Please refer to the Rust docs website for the full documentation*

[![Crates.io](https://img.shields.io/crates/v/as2org-rs)](https://crates.io/crates/as2org-rs)
[![Docs.rs](https://docs.rs/as2org-rs/badge.svg)](https://docs.rs/as2org-rs)
[![License](https://img.shields.io/crates/l/as2org-rs)](https://raw.githubusercontent.com/bgpkit/as2org-rs/main/LICENSE)

as2org-rs: Access CAIDA AS-to-Organization mappings in Rust

This crate provides a small, dependency-light helper for reading and querying
CAIDA's AS Organizations dataset. It downloads (or opens a local/remote path)
the newline-delimited JSON (JSONL) files published by CAIDA and exposes a
simple API to:

- Fetch the latest dataset URL from CAIDA
- Load the dataset into memory
- Look up information for a given ASN
- Find all "sibling" ASNs that belong to the same organization
- Test whether two ASNs are siblings (belong to the same org)

The crate supports local files, HTTP(S) URLs, and gz-compressed inputs via
the `oneio` crate.

### Installation

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
as2org-rs = "1"
```

### Data source
- CAIDA AS Organizations Dataset: <http://www.caida.org/data/as-organizations>

### Data model

Public return type:

`As2orgAsInfo` contains:
- `asn`: the AS number
- `name`: the name provided for the individual AS number
- `country_code`: the registration country code of the organization
- `org_id`: the CAIDA/WHOIS organization identifier
- `org_name`: the organization's name
- `source`: the RIR or NIR database that contained this entry

### Quickstart

Load the most recent dataset and run typical queries:

```rust
use as2org_rs::As2org;

// Construct from the latest public dataset (requires network access)
let as2org = As2org::new(None).unwrap();

// Look up one ASN
let info = as2org.get_as_info(15169).unwrap();
assert_eq!(info.org_id.is_empty(), false);

// List all siblings for an ASN (ASNs under the same org)
let siblings = as2org.get_siblings(15169).unwrap();
assert!(siblings.iter().any(|s| s.asn == 36040));

// Check whether two ASNs are siblings
assert!(as2org.are_siblings(15169, 36040));
```

### Offline and custom input

You can also point to a local file path or a remote URL (HTTP/HTTPS), gzipped
or plain:

```rust
use as2org_rs::As2org;

// From a local jsonl.gz file
let as2org = As2org::new(Some("/path/to/20250101.as-org2info.jsonl.gz".into())).unwrap();

// From an explicit HTTPS URL
let as2org = As2org::new(Some("https://publicdata.caida.org/datasets/as-organizations/20250101.as-org2info.jsonl.gz".into())).unwrap();
```

### Errors

Constructors and helper functions return `anyhow::Result<T>`. For lookups,
the API returns `Option<_>` when a requested ASN or organization is missing.

### Notes

- Network access is only required when you pass `None` to `As2org::new` so the
  crate can discover and fetch the latest dataset URL.
- Dataset files can be large; loading them will allocate in-memory maps for
  fast queries.
- This crate is not affiliated with CAIDA. Please review CAIDA's data usage
  policies before redistribution or heavy automated access.

## License

MIT
