# Changelog

All notable changes to this project will be documented in this file.

## v0.1.0 -- 2024-06-24

Initial release of `as2org-rs`.

### Highlights

* automatically retrieve the most up-to-date CAIDA as-to-organization data
* `.get_as_info(ASN)` to retrieve information about an AS
* `.get_siblings(ASN)` to retrieve all siblings of an AS
* `.are_siblings(ASN1, ASN2)` to check if two ASes are siblings

The main returning data structure is `As2orgAsInfo`, which contains the following fields:

* `asn`: the AS number
* `name`: the name provide for the individual AS number
* `country_code`: the country code of the organization's registration country
* `org_id`: maps to an organization entry
* `org_name`: the name of the organization
* `source`: the RIR or NIR database which was contained this entry
