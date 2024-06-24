use as2org_rs::As2org;

fn main() {
    let as2org = As2org::new(None).unwrap();
    dbg!(as2org.get_siblings(15169).unwrap());
    assert!(as2org.are_siblings(15169, 36040));
}
