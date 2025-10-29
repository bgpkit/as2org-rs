//! # CAIDA as2org utility.
//!
//! ## Data source
//! * The CAIDA [AS Organizations Dataset](http://www.caida.org/data/as-organizations).
//!
//! ## Data structure
//!
//! `As2orgAsInfo`:
//! * `asn`: the AS number
//! * `name`: the name provide for the individual AS number
//! * `country_code`: the country code of the organization's registration country
//! * `org_id`: maps to an organization entry
//! * `org_name`: the name of the organization
//! * `source`: the RIR or NIR database which was contained this entry
//!
//! ## Examples
//!
//! ```rust
//! use as2org_rs::As2org;
//!
//! let as2org = As2org::new(None).unwrap();
//! dbg!(as2org.get_as_info(400644).unwrap());
//! dbg!(as2org.get_siblings(15169).unwrap());
//! assert!(as2org.are_siblings(15169, 36040));
//! ```

use anyhow::{anyhow, Result};
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
pub struct As2orgAsInfo {
    pub asn: u32,
    pub name: String,
    pub country_code: String,
    pub org_id: String,
    pub org_name: String,
    pub source: String,
}

pub struct As2org {
    as_map: HashMap<u32, As2orgJsonAs>,
    org_map: HashMap<String, As2orgJsonOrg>,
    as_to_org: HashMap<u32, String>,
    org_to_as: HashMap<String, Vec<u32>>,
}

const BASE_URL: &str = "https://publicdata.caida.org/datasets/as-organizations";

impl As2org {
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

/// Get the most recent AS2Org data file from CAIDA
fn get_most_recent_data() -> Result<String> {
    let data_link: Regex = Regex::new(r".*(\d{8}\.as-org2info\.jsonl\.gz).*")?;
    let content = oneio::read_to_string(BASE_URL)?;
    let res: Vec<String> = data_link
        .captures_iter(content.as_str())
        .map(|cap| cap[1].to_owned())
        .collect();
    let file = res.last().unwrap().to_string();

    Ok(format!("{BASE_URL}/{file}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_entries() {
        let as2org = As2org::new(None).unwrap();
        dbg!(as2org.get_as_info(400644));
        dbg!(as2org.get_siblings(400644));
        dbg!(as2org.get_siblings(13335));
        dbg!(as2org.get_siblings(61786));
    }
}
