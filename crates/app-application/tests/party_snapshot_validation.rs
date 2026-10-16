use app_application::models::party::{PartyCharacterSnapshot, PartySnapshot};
use serde_json::json;
use uuid::Uuid;

#[test]
fn party_snapshot_rejects_duplicate_members_and_mismatched_document_identity() {
    let host = Uuid::new_v4();
    let guest = Uuid::new_v4();
    let sheet = |id: Uuid| json!({"characterId":id,"name":"Mira","heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":14,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[]});
    let member = PartyCharacterSnapshot {
        member_id: Uuid::new_v4(),
        character_id: guest,
        character: sheet(guest),
    };
    let mut snapshot = PartySnapshot {
        host_member_id: Uuid::new_v4(),
        members: vec![member.clone()],
    };
    assert!(snapshot.validate(&sheet(host)).is_ok());
    snapshot.members.push(member);
    assert!(snapshot.validate(&sheet(host)).is_err());
    snapshot.members.pop();
    snapshot.members[0].character["characterId"] = json!(host);
    assert!(snapshot.validate(&sheet(host)).is_err());
}
