//! Knowledge-only artifact staleness and divergence evaluation.

use std::fs;

use postproject_core::{
    Activity, ActivityId, ActivityInput, ActivityKind, ActivityOutput, ArtifactEvaluationLimits,
    ArtifactKnowledgeReason, ArtifactKnowledgeState, RepresentationFingerprint,
    ResourceFingerprint,
};
use postproject_media::{fingerprint_representation, prepare_original_media};
use postproject_storage_sqlite::SqliteProduction;

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one test follows the complete current-to-stale-to-diverged lifecycle"
)]
fn changed_source_propagates_staleness_and_changed_output_diverges() {
    let directory = tempfile::tempdir().expect("create fixture directory");
    let production_path = directory.path().join("artifacts.pproj");
    let paths = ["source.mov", "proxy.mov", "render.mov"].map(|name| directory.path().join(name));
    for (index, path) in paths.iter().enumerate() {
        fs::write(path, format!("media-{index}")).expect("write media fixture");
    }
    let imports =
        paths.map(|path| prepare_original_media(path, None, None).expect("prepare media fixture"));
    let source_id = imports[0].representation().id();
    let proxy_id = imports[1].representation().id();
    let render_id = imports[2].representation().id();

    let mut production = SqliteProduction::create(&production_path, None).expect("create");
    for import in &imports {
        let mut transaction = production.begin_transaction().expect("begin import");
        transaction.import_original(import).expect("stage import");
        transaction.commit().expect("commit import");
    }
    create_activity(
        &mut production,
        source_id,
        proxy_id,
        "org.postproject:generate-proxy",
    );
    create_activity(
        &mut production,
        proxy_id,
        render_id,
        "org.postproject:render",
    );

    assert_eq!(
        evaluate(&production, proxy_id).state(),
        ArtifactKnowledgeState::Current
    );
    assert_eq!(
        evaluate(&production, render_id).state(),
        ArtifactKnowledgeState::Current
    );

    let source_resource = imports[0].resources()[0].id();
    let original = &imports[0].resources()[0].fingerprints()[0];
    let changed = ResourceFingerprint::new(
        original.algorithm(),
        original.version(),
        vec![0x55; original.value().len()],
    )
    .expect("changed resource fingerprint");
    let mut transaction = production.begin_transaction().expect("begin observation");
    transaction
        .record_resource_fingerprint(source_resource, &changed)
        .expect("record resource fingerprint");
    transaction.commit().expect("commit resource fingerprint");
    drop(transaction);

    let proxy = evaluate(&production, proxy_id);
    assert_eq!(proxy.state(), ArtifactKnowledgeState::Stale);
    assert!(proxy.reasons().iter().any(|reason| matches!(
        reason,
        ArtifactKnowledgeReason::FingerprintRecomputationPending {
            representation_id,
            ..
        } if *representation_id == source_id
    )));
    let render = evaluate(&production, render_id);
    assert_eq!(render.state(), ArtifactKnowledgeState::Stale);
    assert!(render.reasons().iter().any(|reason| matches!(
        reason,
        ArtifactKnowledgeReason::UpstreamNotCurrent {
            representation_id,
            state: ArtifactKnowledgeState::Stale,
        } if *representation_id == proxy_id
    )));

    let source = &production
        .representations(imports[0].asset().id())
        .expect("load source representation")[0];
    let resources = production
        .resources(source_id)
        .expect("load source resources");
    let aggregate = fingerprint_representation(source.content_structure(), &resources)
        .expect("recompute representation fingerprint");
    let mut transaction = production.begin_transaction().expect("begin recompute");
    transaction
        .record_representation_fingerprint(source_id, &aggregate)
        .expect("record representation fingerprint");
    transaction.commit().expect("commit recomputation");
    drop(transaction);
    assert!(
        evaluate(&production, proxy_id)
            .reasons()
            .iter()
            .any(|reason| matches!(reason, ArtifactKnowledgeReason::FingerprintChanged { .. }))
    );

    let proxy_fingerprint = &imports[1].representation().fingerprints()[0];
    let diverged = RepresentationFingerprint::new(
        proxy_fingerprint.algorithm(),
        proxy_fingerprint.version(),
        vec![0x77; proxy_fingerprint.value().len()],
    )
    .expect("diverged fingerprint");
    let mut transaction = production.begin_transaction().expect("begin divergence");
    transaction
        .record_representation_fingerprint(proxy_id, &diverged)
        .expect("record divergence");
    transaction.commit().expect("commit divergence");
    drop(transaction);
    assert_eq!(
        evaluate(&production, proxy_id).state(),
        ArtifactKnowledgeState::Diverged
    );
    assert_eq!(
        evaluate(&production, render_id).state(),
        ArtifactKnowledgeState::Stale
    );
}

fn create_activity(
    production: &mut SqliteProduction,
    input: postproject_core::RepresentationId,
    output: postproject_core::RepresentationId,
    kind: &str,
) {
    let activity = Activity::new(
        ActivityId::new(),
        ActivityKind::new(kind).expect("valid activity kind"),
        vec![ActivityInput::new(input, None)],
        vec![ActivityOutput::new(output, None)],
    )
    .expect("valid activity");
    let mut transaction = production.begin_transaction().expect("begin activity");
    transaction
        .create_activity(&activity)
        .expect("stage activity");
    transaction.commit().expect("commit activity");
}

fn evaluate(
    production: &SqliteProduction,
    representation_id: postproject_core::RepresentationId,
) -> postproject_core::ArtifactEvaluation {
    production
        .evaluate_artifact(representation_id, ArtifactEvaluationLimits::default())
        .expect("evaluate artifact")
}
