use packflow_core::auth::LocalAuth;
#[test]
fn local_account_setup_login_remember_recovery_and_revocation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("account.sqlite");
    let mut a = LocalAuth::open(&path).unwrap();
    assert!(a.require().is_err());
    let recovery = a.setup("owner", "Owner", "A long password 123").unwrap();
    assert!(a.require().is_err());
    assert!(a.setup("again", "Again", "A long password 123").is_err());
    assert!(a.login("owner", "wrong").is_err());
    a.login("owner", "A long password 123").unwrap();
    a.require().unwrap();
    let token = a.remember().unwrap();
    drop(a);
    let mut a = LocalAuth::open(&path).unwrap();
    assert!(a.require().is_err());
    a.resume(&token).unwrap();
    a.logout().unwrap();
    assert!(a.resume(&token).is_err());
    a.login("owner", "A long password 123").unwrap();
    let token = a.remember().unwrap();
    a.change_password("A long password 123", "New long password 456")
        .unwrap();
    assert!(a.require().is_err());
    assert!(a.resume(&token).is_err());
    assert!(a.login("owner", "A long password 123").is_err());
    let new_recovery = a.recover(&recovery, "Recovered password 789").unwrap();
    assert_ne!(recovery, new_recovery);
    assert!(a.recover(&recovery, "Recovered password 789").is_err());
    a.login("owner", "Recovered password 789").unwrap();
    let disk = std::fs::read(path).unwrap();
    assert!(!disk.windows(22).any(|w| w == b"Recovered password 789"));
}
#[test]
fn login_attempt_limit_survives_restart_and_does_not_unlock() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("account.sqlite");
    let mut a = LocalAuth::open(&path).unwrap();
    a.setup("owner", "Owner", "A long password 123").unwrap();
    for _ in 0..5 {
        assert!(a.login("owner", "wrong").is_err());
    }
    drop(a);
    let mut a = LocalAuth::open(&path).unwrap();
    assert_eq!(
        a.login("owner", "A long password 123").unwrap_err().code,
        "RATE_LIMITED"
    );
    assert!(a.require().is_err());
}
