//! Wire-format contract between the HF backend response and the frontend
//! `HfModel` TypeScript interface (`src/api/hf.ts`).
//!
//! HF's own API uses `id`, `lastModified`, `rfilename`; the frontend reads
//! `repo_id`, `last_modified`, `siblings[].filename`. The struct must
//! DESERIALIZE the HF names (it parses HF responses) but SERIALIZE the
//! frontend names (it is returned verbatim from `GET /hf/search`).
//!
//! Regression guard for the audit bug: search results rendered as empty cards
//! because `model.repo_id` / `model.last_modified` were `undefined` in JS, and
//! the sort dropdown's `last-modified` value failed to deserialize.

use app_server::hf::types::{HfModel, HfSibling, HfSort};

#[test]
fn hfmodel_serializes_with_frontend_keys() {
    let m = HfModel {
        repo_id: "Qwen/Qwen3-4B".into(),
        likes: 1,
        downloads: 2,
        gated: false,
        tags: vec!["qwen3".into()],
        last_modified: Some("2026-04-01T00:00:00.000Z".into()),
        siblings: vec![HfSibling {
            filename: "model-q4_k_m.gguf".into(),
            size: Some(10),
        }],
    };
    let v: serde_json::Value = serde_json::to_value(&m).expect("serialize");

    assert!(
        v.get("repo_id").is_some(),
        "frontend expects repo_id; got {v}"
    );
    assert!(v.get("id").is_none(), "must not emit HF 'id' key; got {v}");
    assert_eq!(v["repo_id"], "Qwen/Qwen3-4B");

    assert!(
        v.get("last_modified").is_some(),
        "frontend expects last_modified; got {v}"
    );
    assert!(
        v.get("lastModified").is_none(),
        "must not emit HF 'lastModified' key; got {v}"
    );

    assert_eq!(v["siblings"][0]["filename"], "model-q4_k_m.gguf");
    assert!(
        v["siblings"][0].get("rfilename").is_none(),
        "must not emit HF 'rfilename' key; got {v}"
    );
}

#[test]
fn hfmodel_still_deserializes_hf_api_shape() {
    let body = serde_json::json!({
        "id": "Qwen/Qwen3-4B",
        "lastModified": "2026-04-01T00:00:00.000Z",
        "siblings": [{ "rfilename": "model.safetensors", "size": 5 }]
    });
    let m: HfModel = serde_json::from_value(body).expect("deserialize HF shape");
    assert_eq!(m.repo_id, "Qwen/Qwen3-4B");
    assert_eq!(m.last_modified.as_deref(), Some("2026-04-01T00:00:00.000Z"));
    assert_eq!(m.siblings[0].filename, "model.safetensors");
}

#[test]
fn hfmodel_deserializes_tri_state_gated() {
    // HF's `gated` is `false` | `"auto"` | `"manual"`. With `full=true` the
    // list endpoint returns the string forms, which must coerce to a bool
    // (audit: a `"manual"` value 500'd the entire search).
    let manual: HfModel =
        serde_json::from_value(serde_json::json!({ "id": "a/b", "gated": "manual" }))
            .expect("manual");
    assert!(manual.gated);
    let auto: HfModel =
        serde_json::from_value(serde_json::json!({ "id": "a/b", "gated": "auto" })).expect("auto");
    assert!(auto.gated);
    let open: HfModel =
        serde_json::from_value(serde_json::json!({ "id": "a/b", "gated": false })).expect("false");
    assert!(!open.gated);
}

#[test]
fn hfsort_accepts_frontend_tokens() {
    assert_eq!(
        serde_json::from_str::<HfSort>("\"last-modified\"").expect("last-modified"),
        HfSort::LastModified
    );
    assert_eq!(
        serde_json::from_str::<HfSort>("\"downloads\"").expect("downloads"),
        HfSort::Downloads
    );
    assert_eq!(
        serde_json::from_str::<HfSort>("\"likes\"").expect("likes"),
        HfSort::Likes
    );
}
