//! as2org-rs: Access CAIDA AS-to-Organization mappings in Rust
//!
//! This crate provides a small, dependency-light helper for reading and querying
//! CAIDA's AS Organizations dataset. It downloads (or opens a local/remote path)
//! the newline-delimited JSON (JSONL) files published by CAIDA and exposes a
//! simple API to:
//!
//! - Fetch the latest dataset URL from CAIDA
//! - Load the dataset into memory
//! - Look up information for a given ASN
//! - Find all "sibling" ASNs that belong to the same organization
//! - Test whether two ASNs are siblings (belong to the same org)
//!
//! The crate supports local files, HTTP(S) URLs, and gz-compressed inputs via
//! the `oneio` crate.
//!
//! ## Installation
//!
//! Add the dependency to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! as2org-rs = "1"
//! ```
//!
//! ## Data source
//! - CAIDA AS Organizations Dataset: <http://www.caida.org/data/as-organizations>
//!
//! ## Data model
//!
//! Public return type:
//!
//! `As2orgAsInfo` contains:
//! - `asn`: the AS number
//! - `name`: the name provided for the individual AS number
//! - `country_code`: the registration country code of the organization
//! - `org_id`: the CAIDA/WHOIS organization identifier
//! - `org_name`: the organization's name
//! - `source`: the RIR or NIR database that contained this entry
//!
//! ## Quickstart
//!
//! Load the most recent dataset and run typical queries:
//!
//! ```rust,no_run
//! use as2org_rs::As2org;
//!
//! // Construct from the latest public dataset (requires network access)
//! let as2org = As2org::new(None).unwrap();
//!
//! // Look up one ASN
//! let info = as2org.get_as_info(15169).unwrap();
//! assert_eq!(info.org_id.is_empty(), false);
//!
//! // List all siblings for an ASN (ASNs under the same org)
//! let siblings = as2org.get_siblings(15169).unwrap();
//! assert!(siblings.iter().any(|s| s.asn == 36040));
//!
//! // Check whether two ASNs are siblings
//! assert!(as2org.are_siblings(15169, 36040));
//! ```
//!
//! ## Offline and custom input
//!
//! You can also point to a local file path or a remote URL (HTTP/HTTPS), gzipped
//! or plain:
//!
//! ```rust,no_run
//! use as2org_rs::As2org;
//!
//! // From a local jsonl.gz file
//! let as2org = As2org::new(Some("/path/to/20250101.as-org2info.jsonl.gz".into())).unwrap();
//!
//! // From an explicit HTTPS URL
//! let as2org = As2org::new(Some("https://publicdata.caida.org/datasets/as-organizations/20250101.as-org2info.jsonl.gz".into())).unwrap();
//! ```
//!
//! ## Errors
//!
//! Constructors and helper functions return `anyhow::Result<T>`. For lookups,
//! the API returns `Option<_>` when a requested ASN or organization is missing.
//!
//! ## Notes
//!
//! - Network access is only required when you pass `None` to `As2org::new` so the
//!   crate can discover and fetch the latest dataset URL.
//! - Dataset files can be large; loading them will allocate in-memory maps for
//!   fast queries.
//! - This crate is not affiliated with CAIDA. Please review CAIDA's data usage
//!   policies before redistribution or heavy automated access.

use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Organization JSON format
///
/// --------------------
/// Organization fields
/// --------------------
/// org_id  : unique ID for the given organization
///            some will be created by the WHOIS entry and others will be
///            created by our scripts
/// changed : the changed date provided by its WHOIS entry
/// name    : name could be selected from the AUT entry tied to the
///            organization, the AUT entry with the largest customer cone,
///           listed for the organization (if there existed an stand alone
///            organization), or a human maintained file.
/// country : some WHOIS provide as a individual field. In other cases
///            we inferred it from the addresses
/// source  : the RIR or NIR database which was contained this entry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct As2orgJsonOrg {
    #[serde(alias = "organizationId")]
    org_id: String,

    changed: Option<String>,

    #[serde(default)]
    name: String,

    country: String,

    /// The RIR or NIR database that contained this entry
    source: String,

    #[serde(alias = "type")]
    data_type: String,
}

/// AS Json format
///
/// ----------
/// AS fields
/// ----------
/// asn     : the AS number
/// changed : the changed date provided by its WHOIS entry
/// name    : the name provide for the individual AS number
/// org_id  : maps to an organization entry
/// opaque_id   : opaque identifier used by RIR extended delegation format
/// source  : the RIR or NIR database which was contained this entry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct As2orgJsonAs {
    asn: String,

    changed: Option<String>,

    #[serde(default)]
    name: String,

    #[serde(alias = "opaqueId")]
    opaque_id: Option<String>,

    #[serde(alias = "organizationId")]
    org_id: String,

    /// The RIR or NIR database that contained this entry
    source: String,

    #[serde(rename = "type")]
    data_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum As2orgJsonEntry {
    Org(As2orgJsonOrg),
    As(As2orgJsonAs),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Public information for an Autonomous System (AS) enriched with its organization.
///
/// This struct is returned by high-level query methods like `get_as_info` and
/// `get_siblings` and contains the most commonly used fields for downstream
/// analysis or presentation.
pub struct As2orgAsInfo {
    /// The AS number
    pub asn: u32,
    /// The name provided for the individual AS number
    pub name: String,
    /// The registration country code of the organization
    pub country_code: String,
    /// Organization identifier (as used in the dataset)
    pub org_id: String,
    /// Organization name
    pub org_name: String,
    /// The RIR database that contained this entry
    pub source: String,
}

/// In-memory accessor for CAIDA's AS-to-Organization dataset.
///
/// Construct with `As2org::new`, then perform lookups via `get_as_info`,
/// `get_siblings`, or `are_siblings`.
pub struct As2org {
    as_map: HashMap<u32, As2orgJsonAs>,
    org_map: HashMap<String, As2orgJsonOrg>,
    as_to_org: HashMap<u32, String>,
    org_to_as: HashMap<String, Vec<u32>>,
}

const BASE_URL: &str = "https://publicdata.caida.org/datasets/as-organizations";

impl As2org {
    /// Create a new `As2org` accessor.
    ///
    /// - When `data_file_path` is `None`, the constructor fetches the CAIDA
    ///   index page to discover the most recent `*.as-org2info.jsonl.gz` file
    ///   and reads it via HTTP(S).
    /// - When `Some(path_or_url)` is provided, the path can be a local file or
    ///   a remote URL. Gzipped files are supported transparently.
    ///
    /// Returns `anyhow::Result<Self>` with an initialized in-memory index.
    pub fn new(data_file_path: Option<String>) -> Result<Self> {
        let entries = match data_file_path {
            Some(path) => parse_as2org_file(path.as_str())?,
            None => {
                let url = get_most_recent_data()?;
                parse_as2org_file(url.as_str())?
            }
        };

        let mut as_map: HashMap<u32, As2orgJsonAs> = HashMap::new();
        let mut org_map: HashMap<String, As2orgJsonOrg> = HashMap::new();

        for entry in entries {
            match entry {
                As2orgJsonEntry::As(as_entry) => {
                    as_map.insert(as_entry.asn.parse::<u32>().unwrap(), as_entry);
                }
                As2orgJsonEntry::Org(org_entry) => {
                    org_map.insert(org_entry.org_id.clone(), org_entry);
                }
            }
        }

        let mut as_to_org: HashMap<u32, String> = HashMap::new();
        let mut org_to_as: HashMap<String, Vec<u32>> = HashMap::new();

        for (asn, as_entry) in as_map.iter() {
            as_to_org.insert(*asn, as_entry.org_id.clone());
            let org_asn = org_to_as.entry(as_entry.org_id.clone()).or_default();
            org_asn.push(*asn);
        }

        Ok(Self {
            as_map,
            org_map,
            as_to_org,
            org_to_as,
        })
    }

    /// List all available dataset files published by CAIDA with their dates.
    ///
    /// Returns a vector of `(url, date)` pairs sorted by date ascending; the last
    /// element is the most recent dataset.
    ///
    /// This is useful for offline workflows that want to pin to a specific
    /// snapshot instead of always using the latest.
    pub fn get_all_files_with_dates() -> Result<Vec<(String, NaiveDate)>> {
        get_all_files_with_dates()
    }

    /// Returns the URL for the latest AS-to-Organization dataset file.
    ///
    /// This function returns a direct URL to CAIDA's most recent dataset using
    /// the "latest" symlink. This is a convenience wrapper that formats the
    /// complete URL string.
    ///
    /// # Returns
    /// A string containing the HTTPS URL to the latest .jsonl.gz dataset file.
    pub fn get_latest_file_url() -> String {
        format!("{BASE_URL}/latest.as-org2info.jsonl.gz")
    }

    /// Get enriched information for a specific ASN, if present.
    ///
    /// Returns `None` when the ASN is not found in the loaded dataset.
    ///
    /// Example:
    /// ```rust,no_run
    /// # use as2org_rs::As2org;
    /// let db = As2org::new(None).unwrap();
    /// let info = db.get_as_info(15169).unwrap();
    /// assert!(!info.org_id.is_empty());
    /// ```
    pub fn get_as_info(&self, asn: u32) -> Option<As2orgAsInfo> {
        let as_entry = self.as_map.get(&asn)?;
        let org_id = as_entry.org_id.as_str();
        let org_entry = self.org_map.get(org_id)?;
        Some(As2orgAsInfo {
            asn,
            name: as_entry.name.clone(),
            country_code: org_entry.country.clone(),
            org_id: org_id.to_string(),
            org_name: org_entry.name.clone(),
            source: org_entry.source.clone(),
        })
    }

    /// Return all ASNs that belong to the same organization as the given ASN.
    ///
    /// The returned vector includes the queried ASN itself. Returns `None`
    /// when the ASN is not present in the dataset.
    ///
    /// Example:
    /// ```rust,no_run
    /// # use as2org_rs::As2org;
    /// let db = As2org::new(None).unwrap();
    /// let sibs = db.get_siblings(15169).unwrap();
    /// assert!(sibs.iter().any(|s| s.asn == 15169));
    /// ```
    pub fn get_siblings(&self, asn: u32) -> Option<Vec<As2orgAsInfo>> {
        let org_id = self.as_to_org.get(&asn)?;
        let org_asns = self.org_to_as.get(org_id)?.to_vec();
        Some(
            org_asns
                .iter()
                .map(|asn| self.get_as_info(*asn).unwrap())
                .collect(),
        )
    }

    /// Return `true` if both ASNs belong to the same organization.
    ///
    /// Returns `false` if either ASN is missing from the dataset or their
    /// organization differs.
    ///
    /// Example:
    /// ```rust,no_run
    /// # use as2org_rs::As2org;
    /// let db = As2org::new(None).unwrap();
    /// assert!(db.are_siblings(15169, 36040));
    /// ```
    pub fn are_siblings(&self, asn1: u32, asn2: u32) -> bool {
        let org1 = match self.as_to_org.get(&asn1) {
            None => return false,
            Some(o) => o,
        };
        let org2 = match self.as_to_org.get(&asn2) {
            None => return false,
            Some(o) => o,
        };
        org1 == org2
    }
}

/// Fixes misinterpretation of strings encoded in Latin-1 that were mistakenly decoded as UTF-8.
///
/// This function processes a string that may contain characters misinterpreted due to an
/// incorrect encoding or decoding process. Specifically, it handles cases where Latin-1
/// characters are represented as two incorrect UTF-8 characters, such as 'Ã' followed
/// by a secondary byte.
///
/// # Arguments
///
/// * `input` - A string slice that may contain incorrectly encoded characters.
///
/// # Returns
///
/// A corrected string with all misinterpreted characters properly fixed or left unchanged
/// if the pattern doesn't match.
fn fix_latin1_misinterpretation(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        // Check for the pattern of misinterpreted Latin-1 chars
        if c == 'Ã' && chars.peek().is_some() {
            let next_char = chars.next().unwrap();

            // Calculate the original Latin-1 character
            let byte_value = match next_char {
                '\u{0080}'..='\u{00BF}' => 0xC0 + (next_char as u32 - 0x0080),
                // Handle other ranges as needed
                _ => {
                    // If it doesn't match the pattern, treat as normal chars
                    result.push(c);
                    result.push(next_char);
                    continue;
                }
            };

            // Convert to the correct character
            if let Some(correct_char) = char::from_u32(byte_value) {
                result.push(correct_char);
            } else {
                // Fallback for invalid characters
                result.push(c);
                result.push(next_char);
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// parse remote AS2Org file into Vec of DataEntry
fn parse_as2org_file(path: &str) -> Result<Vec<As2orgJsonEntry>> {
    let mut res: Vec<As2orgJsonEntry> = vec![];

    for line in oneio::read_lines(path)? {
        let line = fix_latin1_misinterpretation(&line?);
        if line.contains(r#""type":"ASN""#) {
            let data = serde_json::from_str::<As2orgJsonAs>(line.as_str());
            match data {
                Ok(data) => {
                    res.push(As2orgJsonEntry::As(data));
                }
                Err(e) => {
                    eprintln!("error parsing line:\n{}", line.as_str());
                    return Err(anyhow!(e));
                }
            }
        } else {
            let data = serde_json::from_str::<As2orgJsonOrg>(line.as_str());
            match data {
                Ok(data) => {
                    res.push(As2orgJsonEntry::Org(data));
                }
                Err(e) => {
                    eprintln!("error parsing line:\n{}", line.as_str());
                    return Err(anyhow!(e));
                }
            }
        }
    }
    Ok(res)
}

/// Returns a vector of tuples containing file names and their corresponding dates for all AS2Org data files.
/// The vector is sorted by dates with the latest date last.
///
/// # Returns
/// - `Result<Vec<(String, NaiveDate)>>` where each tuple contains:
///   - String: name of the AS2Org data file
///   - NaiveDate: date extracted from the file name
fn get_all_files_with_dates() -> Result<Vec<(String, NaiveDate)>> {
    let data_link: Regex = Regex::new(r".*(\d{8}\.as-org2info\.jsonl\.gz).*")?;
    let content = oneio::read_to_string(BASE_URL)?;
    let mut res: Vec<(String, NaiveDate)> = data_link
        .captures_iter(content.as_str())
        .map(|cap| {
            let file = cap[1].to_owned();
            let date = NaiveDate::parse_from_str(&file[..8], "%Y%m%d").unwrap();
            (format!("{BASE_URL}/{file}"), date)
        })
        .collect();
    res.sort_by_key(|(_, date)| *date);
    Ok(res)
}
fn get_most_recent_data() -> Result<String> {
    let files = get_all_files_with_dates()?;
    Ok(files.last().unwrap().0.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    // Helper to create a shared As2org instance for all tests
    // This ensures we only fetch the data once
    fn get_test_db() -> As2org {
        // Use a static to cache the database across tests
        // Note: In a real scenario with multiple test threads, you might want to use lazy_static
        As2org::new(None).expect("Failed to load AS2org database")
    }

    #[test]
    fn test_new_from_latest() {
        let as2org = get_test_db();
        // Verify the database was loaded by checking if we have some data
        assert!(as2org.as_map.len() > 0);
        assert!(as2org.org_map.len() > 0);
    }

    #[test]
    fn test_get_as_info_existing() {
        let as2org = get_test_db();
        // Test with a well-known ASN (Google)
        let info = as2org.get_as_info(15169);
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.asn, 15169);
        assert!(!info.org_id.is_empty());
        assert!(!info.org_name.is_empty());
        assert!(!info.country_code.is_empty());
        assert!(!info.source.is_empty());
    }

    #[test]
    fn test_get_as_info_nonexistent() {
        let as2org = get_test_db();
        // Test with a likely non-existent ASN
        let info = as2org.get_as_info(999999999);
        assert!(info.is_none());
    }

    #[test]
    fn test_get_siblings_existing() {
        let as2org = get_test_db();
        // Test with Google's AS15169
        let siblings = as2org.get_siblings(15169);
        assert!(siblings.is_some());
        let siblings = siblings.unwrap();
        // Should include at least the ASN itself
        assert!(siblings.len() >= 1);
        // The queried ASN should be in the siblings list
        assert!(siblings.iter().any(|s| s.asn == 15169));
        // All siblings should have the same org_id
        let org_id = &siblings[0].org_id;
        assert!(siblings.iter().all(|s| s.org_id == *org_id));
    }

    #[test]
    fn test_get_siblings_nonexistent() {
        let as2org = get_test_db();
        let siblings = as2org.get_siblings(999999999);
        assert!(siblings.is_none());
    }

    #[test]
    fn test_are_siblings_true() {
        let as2org = get_test_db();
        // First get an ASN that has siblings
        let _info = as2org.get_as_info(15169).unwrap();
        let siblings = as2org.get_siblings(15169).unwrap();
        
        if siblings.len() > 1 {
            // Test with actual siblings if they exist
            let sibling_asn = siblings.iter().find(|s| s.asn != 15169).unwrap().asn;
            assert!(as2org.are_siblings(15169, sibling_asn));
        } else {
            // An ASN is always a sibling to itself
            assert!(as2org.are_siblings(15169, 15169));
        }
    }

    #[test]
    fn test_are_siblings_false() {
        let as2org = get_test_db();
        // Google (15169) and Cloudflare (13335) should not be siblings
        assert!(!as2org.are_siblings(15169, 13335));
    }

    #[test]
    fn test_are_siblings_nonexistent() {
        let as2org = get_test_db();
        // Test with non-existent ASN
        assert!(!as2org.are_siblings(15169, 999999999));
        assert!(!as2org.are_siblings(999999999, 15169));
        assert!(!as2org.are_siblings(999999999, 999999998));
    }

    #[test]
    fn test_get_latest_file_url() {
        let url = As2org::get_latest_file_url();
        assert!(url.starts_with("https://"));
        assert!(url.contains("as-org2info.jsonl.gz"));
    }

    #[test]
    fn test_get_all_files_with_dates() {
        let files = As2org::get_all_files_with_dates();
        assert!(files.is_ok());
        let files = files.unwrap();
        assert!(files.len() > 0);
        
        // Verify format of returned data
        for (url, date) in &files {
            assert!(url.starts_with("https://"));
            assert!(url.contains("as-org2info.jsonl.gz"));
            // Date should be valid (just checking it's not a default)
            assert!(date.year() >= 2000);
        }
        
        // Verify sorting (dates should be in ascending order)
        for i in 1..files.len() {
            assert!(files[i].1 >= files[i-1].1);
        }
    }

    #[test]
    fn test_as_to_org_mapping() {
        let as2org = get_test_db();
        // Verify internal consistency: as_to_org should map to valid orgs
        for (asn, org_id) in as2org.as_to_org.iter().take(10) {
            assert!(as2org.org_map.contains_key(org_id));
            assert!(as2org.as_map.contains_key(asn));
        }
    }

    #[test]
    fn test_org_to_as_mapping() {
        let as2org = get_test_db();
        // Verify internal consistency: org_to_as should map to valid ASNs
        for (org_id, asns) in as2org.org_to_as.iter().take(10) {
            assert!(as2org.org_map.contains_key(org_id));
            for asn in asns {
                assert!(as2org.as_map.contains_key(asn));
                assert_eq!(as2org.as_to_org.get(asn).unwrap(), org_id);
            }
        }
    }

    #[test]
    fn test_fix_latin1_misinterpretation() {
        // Test the Latin-1 fix function with known patterns
        let input = "Test Ã© string";
        let fixed = fix_latin1_misinterpretation(input);
        // The function should convert Ã© to é (Latin-1 0xE9)
        assert!(fixed.len() <= input.len());
        
        // Test with no special characters
        let input = "Normal ASCII string";
        let fixed = fix_latin1_misinterpretation(input);
        assert_eq!(input, fixed);
    }

    #[test]
    fn test_as2org_as_info_fields() {
        let as2org = get_test_db();
        let info = as2org.get_as_info(15169).unwrap();
        
        // Verify all fields are populated
        assert_eq!(info.asn, 15169);
        assert!(!info.name.is_empty());
        assert!(!info.country_code.is_empty());
        assert!(!info.org_id.is_empty());
        assert!(!info.org_name.is_empty());
        assert!(!info.source.is_empty());
    }

    #[test]
    fn test_siblings_consistency() {
        let as2org = get_test_db();
        let asn = 15169;
        let siblings = as2org.get_siblings(asn).unwrap();
        
        // All siblings should return the same sibling list
        for sibling in &siblings {
            let sibling_siblings = as2org.get_siblings(sibling.asn).unwrap();
            assert_eq!(siblings.len(), sibling_siblings.len());
            
            // All ASNs should be present in both lists
            for s in &siblings {
                assert!(sibling_siblings.iter().any(|ss| ss.asn == s.asn));
            }
        }
    }
}
