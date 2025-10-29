# Changelog

All notable changes to this project will be documented in this file.

## v1.1.0 -- 2025-10-29

### New Features

* Added `As2org::get_all_files_with_dates()` method to list all available dataset files with their dates
* Added `As2org::get_latest_file_url()` method to get the URL for the latest dataset file
* Introduced `BASE_URL` constant for CAIDA dataset location, improving code maintainability

### Improvements

* Enhanced rustdoc with comprehensive examples and usage patterns
* Marked rustdoc examples with `no_run` to prevent unnecessary network calls during doc tests
* Significantly expanded test suite with 15 comprehensive unit tests covering:
    - Database initialization and loading
    - ASN information retrieval (existing and non-existent)
    - Sibling ASN lookups and consistency checks
    - Organization mapping validation
    - Helper function testing
    - Internal data structure consistency
* Improved code documentation and inline comments

### Bug Fixes

* Fixed regex pattern in `get_most_recent_data()` to use proper digit matching (`\d{8}`)
* Refactored URL construction to use `BASE_URL` constant for consistency

## v1.0.0 -- 2025-04-04

This crate is now being used in several production systems, and we now consider this crate stable.

### Highlights

* automatically detect and fix potential latin-1 encoding in parsed Unicode strings

## v0.1.0 -- 2024-06-24

Initial release of `as2org-rs`.

### Highlights

* automatically retrieve the most up-to-date CAIDA as-to-organization data
* `.get_as_info(ASN)` to retrieve information about an AS
* `.get_siblings(ASN)` to retrieve all siblings of an AS
* `.are_siblings(ASN1, ASN2)` to check if two ASes are siblings

The main returning data structure is `As2orgAsInfo`, which contains the following fields:

* `asn`: the AS number
* `name`: the name provided for the individual AS number
* `country_code`: the country code of the organization's registration country
* `org_id`: maps to an organization entry
* `org_name`: the name of the organization
* `source`: the RIR or NIR database which was contained this entry
