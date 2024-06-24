# `as2org-rs`: utility crate for accessing CAIDA AS to organization mapping

*This readme is generated from the library's doc comments using [cargo-readme](https://github.com/livioribeiro/cargo-readme). Please refer to the Rust docs website for the full documentation*

[![Crates.io](https://img.shields.io/crates/v/as2org-rs)](https://crates.io/crates/as2org-rs)
[![Docs.rs](https://docs.rs/as2org-rs/badge.svg)](https://docs.rs/as2org-rs)
[![License](https://img.shields.io/crates/l/as2org-rs)](https://raw.githubusercontent.com/bgpkit/as2org-rs/main/LICENSE)

## CAIDA as2org utility.

### Data source
* The CAIDA [AS Organizations Dataset](http://www.caida.org/data/as-organizations).

### Data structure

`As2orgAsInfo`:
* `asn`: the AS number
* `name`: the name provide for the individual AS number
* `country_code`: the country code of the organization's registration country
* `org_id`: maps to an organization entry
* `org_name`: the name of the organization
* `source`: the RIR or NIR database which was contained this entry

### Examples

```rust
use as2org_rs::As2org;

let as2org = As2org::new(None).unwrap();
dbg!(as2org.get_as_info(400644).unwrap());
dbg!(as2org.get_siblings(15169).unwrap());
assert!(as2org.are_siblings(15169, 36040));
```

## License

MIT
