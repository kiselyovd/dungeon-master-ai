use app_application::campaign_bundles::VerifyBundleMedia;
use app_application::models::media_asset::MediaAssetError;
use app_application::models::{campaign_bundle::*, media_asset::MediaDescriptor};
use app_application::ports::media_assets::MediaContentInspector;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use uuid::Uuid;

fn media() -> BundleMediaEntry {
    let id = Uuid::new_v4();
    BundleMediaEntry {
        asset_id: id,
        name: BundleMediaEntry::opaque_name(id, "image/png").unwrap(),
        descriptor: MediaDescriptor {
            sha256: "a".repeat(64),
            mime_type: "image/png".into(),
            byte_len: 4,
            width: 1,
            height: 1,
        },
        bytes: vec![1, 2, 3, 4],
    }
}
#[test]
fn full_backup_is_only_for_verified_solo_or_host_storage_owner() {
    let actor = Uuid::new_v4();
    for principal in [
        ExportPrincipal::SoloStorageOwner {
            own_actor: Some(actor),
        },
        ExportPrincipal::HostStorageOwner {
            own_actor: Some(actor),
        },
    ] {
        assert_eq!(
            authorize_export(principal, BundleKind::FullCampaignBackup, None),
            Ok(AuthorizedExport::FullCampaignBackup)
        );
    }
    assert_eq!(
        authorize_export(
            ExportPrincipal::Player { actor_id: actor },
            BundleKind::FullCampaignBackup,
            None
        ),
        Err(BundleError::ExportForbidden)
    );
    assert!(authorize_export(
        ExportPrincipal::HostStorageOwner {
            own_actor: Some(actor)
        },
        BundleKind::FullCampaignBackup,
        Some(actor)
    )
    .is_err());
}
#[test]
fn dossier_cannot_target_another_players_private_sheet() {
    let own = Uuid::new_v4();
    for principal in [
        ExportPrincipal::Player { actor_id: own },
        ExportPrincipal::HostStorageOwner {
            own_actor: Some(own),
        },
        ExportPrincipal::SoloStorageOwner {
            own_actor: Some(own),
        },
    ] {
        assert_eq!(
            authorize_export(principal, BundleKind::PlayerDossier, Some(own)),
            Ok(AuthorizedExport::PlayerDossier { actor_id: own })
        );
        assert_eq!(
            authorize_export(principal, BundleKind::PlayerDossier, Some(Uuid::new_v4())),
            Err(BundleError::ExportForbidden)
        );
        assert!(authorize_export(principal, BundleKind::PlayerDossier, None).is_err());
    }
    assert!(authorize_export(
        ExportPrincipal::Player {
            actor_id: Uuid::nil()
        },
        BundleKind::PlayerDossier,
        Some(Uuid::nil())
    )
    .is_err());
}
#[test]
fn dossier_is_not_a_restorable_campaign() {
    assert_eq!(
        require_restorable(BundleKind::PlayerDossier),
        Err(BundleError::NotRestorable)
    );
    assert!(require_restorable(BundleKind::FullCampaignBackup).is_ok());
}
#[test]
fn schema_ruleset_and_campaign_identity_are_explicit() {
    let header = BundleHeader {
        schema_version: 1,
        ruleset_id: "srd-5.2.1".into(),
        source_campaign_id: Uuid::new_v4(),
        exported_at_epoch_ms: 1,
    };
    assert!(header.validate().is_ok());
    assert_eq!(
        BundleHeader {
            schema_version: 2,
            ..header.clone()
        }
        .validate(),
        Err(BundleError::SchemaUnsupported)
    );
    assert_eq!(
        BundleHeader {
            ruleset_id: "srd-2014".into(),
            ..header.clone()
        }
        .validate(),
        Err(BundleError::RulesetMismatch)
    );
    assert!(BundleHeader {
        source_campaign_id: Uuid::nil(),
        ..header
    }
    .validate()
    .is_err());
}
#[test]
fn archive_names_cannot_traverse_link_execute_or_fetch_a_url() {
    for name in [
        "../file.png",
        "/file.png",
        "C:\\file.png",
        "media/../file.png",
        "https://example.invalid/portrait.png",
        "media/script.exe",
        "media/link.symlink",
    ] {
        let mut entry = media();
        entry.name = name.into();
        assert_eq!(entry.validate_structure(), Err(BundleError::MediaInvalid));
    }
    let mut entry = media();
    entry.descriptor.mime_type = "application/x-msdownload".into();
    assert!(entry.validate_structure().is_err());
}
#[test]
fn forged_lengths_dimensions_duplicates_and_expanded_limits_fail() {
    let entry = media();
    assert!(validate_media_structure(std::slice::from_ref(&entry)).is_ok());
    let mut altered = entry.clone();
    altered.descriptor.byte_len = 20;
    assert!(altered.validate_structure().is_err());
    altered = entry.clone();
    altered.descriptor.width = 100_000;
    assert!(altered.validate_structure().is_err());
    assert!(validate_media_structure(&[entry.clone(), entry]).is_err());
    let entries: Vec<_> = (0..=MAX_BUNDLE_MEDIA_ENTRIES).map(|_| media()).collect();
    assert_eq!(
        validate_media_structure(&entries),
        Err(BundleError::LimitExceeded)
    );
    let entries: Vec<_> = (0..3)
        .map(|_| {
            let mut value = media();
            value.descriptor.width = 4096;
            value.descriptor.height = 8192;
            value
        })
        .collect();
    assert_eq!(
        validate_media_structure(&entries),
        Err(BundleError::LimitExceeded)
    );
}
#[test]
fn unknown_wire_fields_do_not_smuggle_settings_or_links() {
    let mut value = serde_json::to_value(media()).unwrap();
    value["symlink"] = "../file".into();
    assert!(serde_json::from_value::<BundleMediaEntry>(value).is_err());
    let value = serde_json::json!({"schema_version":1,"ruleset_id":"srd-5.2.1","source_campaign_id":Uuid::new_v4(),"exported_at_epoch_ms":1,"provider_token":"fixture"});
    assert!(serde_json::from_value::<BundleHeader>(value).is_err());
}

struct Inspector {
    observed: MediaDescriptor,
    calls: Arc<AtomicUsize>,
}
impl MediaContentInspector for Inspector {
    fn inspect(&self, _bytes: &[u8], _mime: &str) -> Result<MediaDescriptor, MediaAssetError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.observed.clone())
    }
    fn verify_decoded(
        &self,
        _bytes: &[u8],
        _descriptor: &MediaDescriptor,
    ) -> Result<(), MediaAssetError> {
        // Contract stub only; actual image decoding is tested in adapter-media.
        Ok(())
    }
    fn decode_inline(
        &self,
        _url: &str,
    ) -> Result<app_application::models::media_asset::InlineMediaData, MediaAssetError> {
        panic!("portable import must not resolve any URL");
    }
}
fn header() -> BundleHeader {
    BundleHeader {
        schema_version: 1,
        ruleset_id: "srd-5.2.1".into(),
        source_campaign_id: Uuid::new_v4(),
        exported_at_epoch_ms: 1,
    }
}
#[test]
fn verified_media_requires_actual_hash_dimensions_and_mime_to_match() {
    let entry = media();
    for mismatch in [
        MediaDescriptor {
            sha256: "b".repeat(64),
            ..entry.descriptor.clone()
        },
        MediaDescriptor {
            width: 2,
            ..entry.descriptor.clone()
        },
        MediaDescriptor {
            mime_type: "image/jpeg".into(),
            ..entry.descriptor.clone()
        },
    ] {
        let inspector = Arc::new(Inspector {
            observed: mismatch,
            calls: Arc::new(AtomicUsize::new(0)),
        });
        assert!(matches!(
            VerifyBundleMedia::new(inspector).execute(&header(), vec![entry.clone()]),
            Err(BundleError::IntegrityMismatch)
        ));
    }
    let inspector = Arc::new(Inspector {
        observed: entry.descriptor.clone(),
        calls: Arc::new(AtomicUsize::new(0)),
    });
    let result = VerifyBundleMedia::new(inspector)
        .execute(&header(), vec![entry.clone()])
        .unwrap();
    assert_eq!(result.entries(), &[entry]);
}
#[test]
fn structural_rejection_happens_before_media_decoding_or_any_url_resolution() {
    let mut entry = media();
    entry.name = "https://example.invalid/file.png".into();
    let calls = Arc::new(AtomicUsize::new(0));
    let inspector = Arc::new(Inspector {
        observed: entry.descriptor.clone(),
        calls: calls.clone(),
    });
    assert!(VerifyBundleMedia::new(inspector)
        .execute(&header(), vec![entry])
        .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
#[test]
fn a_header_only_inspector_cannot_construct_verified_import_media() {
    struct HeaderOnly(MediaDescriptor);
    impl MediaContentInspector for HeaderOnly {
        fn inspect(&self, _: &[u8], _: &str) -> Result<MediaDescriptor, MediaAssetError> {
            Ok(self.0.clone())
        }
    }
    let entry = media();
    assert!(matches!(
        VerifyBundleMedia::new(Arc::new(HeaderOnly(entry.descriptor.clone())))
            .execute(&header(), vec![entry]),
        Err(BundleError::MediaInvalid)
    ));
}
