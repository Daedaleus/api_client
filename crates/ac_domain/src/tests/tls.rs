use super::CertificateCheck::{Inherit, Off, On};
use super::*;

#[test]
fn the_first_setting_that_does_not_inherit_decides_and_checking_is_the_default() {
    assert!(checks_certificates([]));
    assert!(checks_certificates([Inherit, Inherit]));
    assert!(!checks_certificates([Inherit, Off, On]));
    assert!(checks_certificates([On, Off]));
    assert!(!checks_certificates([Off]));
    assert_eq!(CertificateCheck::default(), Inherit);
}
