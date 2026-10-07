use app_domain::party::{PartyError, PartyRoster};
use uuid::Uuid;

#[test]
fn party_uses_stable_member_and_character_ids_for_ownership() {
    let host = Uuid::new_v4();
    let hero = Uuid::new_v4();
    let guest = Uuid::new_v4();
    let guest_hero = Uuid::new_v4();
    let mut party = PartyRoster::new(host, hero).unwrap();
    party.join(guest, guest_hero).unwrap();
    assert!(party.authorize(host, hero).is_ok());
    assert!(party.authorize(guest, guest_hero).is_ok());
    assert_eq!(party.authorize(guest, hero), Err(PartyError::NotOwner));
    assert_eq!(
        party.authorize(Uuid::new_v4(), guest_hero),
        Err(PartyError::UnknownMember)
    );
    assert_eq!(
        party.join(Uuid::new_v4(), hero),
        Err(PartyError::CharacterAlreadyOwned)
    );
}
#[test]
fn reconnect_is_idempotent_but_does_not_allow_character_rebinding() {
    let host = Uuid::new_v4();
    let hero = Uuid::new_v4();
    let guest = Uuid::new_v4();
    let guest_hero = Uuid::new_v4();
    let mut party = PartyRoster::new(host, hero).unwrap();
    party.join(guest, guest_hero).unwrap();
    let old = party.clone();
    party.join(guest, guest_hero).unwrap();
    assert_eq!(party, old);
    assert_eq!(
        party.join(guest, Uuid::new_v4()),
        Err(PartyError::MemberAlreadyBound)
    );
    assert_eq!(party, old);
}
#[test]
fn malformed_identity_or_snapshot_is_rejected() {
    assert_eq!(
        PartyRoster::new(Uuid::nil(), Uuid::new_v4()),
        Err(PartyError::InvalidIdentity)
    );
    let mut party = PartyRoster::new(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    while party.members.len() < 4 {
        party.join(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    }
    assert_eq!(
        party.join(Uuid::new_v4(), Uuid::new_v4()),
        Err(PartyError::Full)
    );
    let mut bad = party.clone();
    bad.members[1].character_id = bad.members[0].character_id;
    assert!(bad.validate().is_err());
    assert_eq!(party.members.len(), 4);
}
