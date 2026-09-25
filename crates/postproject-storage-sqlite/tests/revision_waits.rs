//! Bounded revision waits across threads, handles, and processes.

use std::{
    env, fs,
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use postproject_core::{
    Asset, AssetId, ContentStructure, ErrorKind, Locator, LocatorAvailability, LocatorId,
    MAX_REVISION_PAGE_SIZE, MAX_REVISION_WAIT, OriginalMediaImport, Representation,
    RepresentationId, RepresentationKind, Resource, ResourceId, Revision, RevisionWaitOutcome,
    RevisionWaiter, Timestamp,
};
use postproject_storage_sqlite::SqliteProduction;
use tempfile::tempdir;

const COMMIT_WORKER_PRODUCTION: &str = "POSTPROJECT_TEST_WAIT_COMMIT_PRODUCTION";

fn import(label: u8) -> OriginalMediaImport {
    let asset_id = AssetId::from_bytes([label; 16]);
    let representation_id = RepresentationId::from_bytes([label.wrapping_add(1); 16]);
    let resource_id = ResourceId::from_bytes([label.wrapping_add(2); 16]);
    OriginalMediaImport::new(
        Asset::new(asset_id, Timestamp::from_unix_micros(0), None, None),
        Representation::new(
            representation_id,
            asset_id,
            RepresentationKind::Original,
            ContentStructure::single_resource(resource_id),
            Vec::new(),
        ),
        vec![Resource::new(resource_id, Vec::new(), None)],
        vec![
            Locator::new(
                LocatorId::from_bytes([label.wrapping_add(3); 16]),
                resource_id,
                format!("file:///media/{label}.mov"),
                None,
                LocatorAvailability::Online,
            )
            .expect("valid locator"),
        ],
    )
    .expect("valid import")
}

fn commit_import(production: &mut SqliteProduction, label: u8) {
    let mut transaction = production.begin_transaction().expect("begin import");
    transaction.import_original(&import(label)).expect("import");
    transaction.commit().expect("commit import");
}

fn sequences(outcome: &RevisionWaitOutcome) -> Vec<u64> {
    match outcome {
        RevisionWaitOutcome::Revisions(revisions) => {
            revisions.iter().map(Revision::sequence).collect()
        }
        other => panic!("expected revisions, got {other:?}"),
    }
}

#[test]
fn zero_timeout_checks_once_and_pages_are_bounded() {
    let directory = tempdir().expect("temporary directory");
    let mut production =
        SqliteProduction::create(directory.path().join("waits.pproj"), None).expect("create");
    let mut waiter = production.revision_waiter().expect("create waiter");
    assert_eq!(
        waiter
            .wait_for_revisions(0, 10, Duration::ZERO)
            .expect("check empty journal"),
        RevisionWaitOutcome::TimedOut
    );

    for label in [10, 20, 30] {
        commit_import(&mut production, label);
    }
    let first = waiter
        .wait_for_revisions(0, 2, Duration::ZERO)
        .expect("check journal");
    assert_eq!(sequences(&first), [1, 2]);
    let RevisionWaitOutcome::Revisions(revisions) = &first else {
        unreachable!()
    };
    assert_eq!(revisions, &production.changes_since(0, 2).expect("page"));
    assert_eq!(
        sequences(
            &waiter
                .wait_for_revisions(2, 10, Duration::from_millis(1))
                .expect("wait after two")
        ),
        [3]
    );
    assert_eq!(
        waiter
            .wait_for_revisions(3, 10, Duration::from_millis(1))
            .expect("wait after latest"),
        RevisionWaitOutcome::TimedOut
    );
    assert_eq!(
        waiter
            .wait_for_revisions(u64::MAX, 10, Duration::ZERO)
            .expect("unrepresentable cursor"),
        RevisionWaitOutcome::TimedOut
    );

    for (limit, timeout) in [
        (0, Duration::ZERO),
        (MAX_REVISION_PAGE_SIZE + 1, Duration::ZERO),
        (1, MAX_REVISION_WAIT + Duration::from_millis(1)),
    ] {
        assert_eq!(
            waiter
                .wait_for_revisions(0, limit, timeout)
                .expect_err("invalid wait")
                .kind(),
            ErrorKind::InvalidArgument
        );
    }
}

#[test]
fn commits_through_the_production_wake_a_blocked_waiter() {
    let directory = tempdir().expect("temporary directory");
    let mut production =
        SqliteProduction::create(directory.path().join("waits.pproj"), None).expect("create");
    let mut waiter = production.revision_waiter().expect("create waiter");
    let blocked = thread::spawn(move || {
        RevisionWaiter::wait_for_revisions(&mut waiter, 0, 10, MAX_REVISION_WAIT)
            .expect("wait for commit")
    });
    commit_import(&mut production, 10);
    assert_eq!(sequences(&blocked.join().expect("join waiter")), [1]);
}

#[test]
fn commits_through_another_handle_are_observed() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("waits.pproj");
    let production = SqliteProduction::create(&path, None).expect("create");
    let mut waiter = production.revision_waiter().expect("create waiter");
    let blocked = thread::spawn(move || {
        waiter
            .wait_for_revisions(0, 10, MAX_REVISION_WAIT)
            .expect("wait")
    });
    let mut other = SqliteProduction::open(&path).expect("open second handle");
    commit_import(&mut other, 10);
    assert_eq!(sequences(&blocked.join().expect("join waiter")), [1]);
    drop(production);
}

#[test]
fn closing_the_production_releases_waiters_permanently() {
    let directory = tempdir().expect("temporary directory");
    let production =
        SqliteProduction::create(directory.path().join("waits.pproj"), None).expect("create");
    let mut waiter = production.revision_waiter().expect("create waiter");
    let blocked = thread::spawn(move || {
        let outcome = waiter
            .wait_for_revisions(0, 10, MAX_REVISION_WAIT)
            .expect("wait until closed");
        (outcome, waiter)
    });
    drop(production);
    let (outcome, mut waiter) = blocked.join().expect("join waiter");
    assert_eq!(outcome, RevisionWaitOutcome::Closed);
    assert_eq!(
        waiter
            .wait_for_revisions(0, 10, MAX_REVISION_WAIT)
            .expect("wait after close"),
        RevisionWaitOutcome::Closed
    );

    let production =
        SqliteProduction::open(directory.path().join("waits.pproj")).expect("reopen production");
    let mut waiter = production.revision_waiter().expect("create waiter");
    production.close_revision_waiters();
    assert_eq!(
        waiter
            .wait_for_revisions(0, 10, MAX_REVISION_WAIT)
            .expect("wait after explicit close"),
        RevisionWaitOutcome::Closed
    );
}

#[test]
fn cancelling_a_waiter_is_terminal_and_isolated() {
    let directory = tempdir().expect("temporary directory");
    let mut production =
        SqliteProduction::create(directory.path().join("waits.pproj"), None).expect("create");
    let mut target = production.revision_waiter().expect("create waiter");
    let mut unaffected = production.revision_waiter().expect("create second waiter");
    let canceller = target.canceller();
    let blocked = thread::spawn(move || {
        let outcome = target
            .wait_for_revisions(0, 10, MAX_REVISION_WAIT)
            .expect("wait until cancelled");
        (outcome, target)
    });
    canceller.cancel();
    let (outcome, mut target) = blocked.join().expect("join waiter");
    assert_eq!(outcome, RevisionWaitOutcome::Cancelled);

    commit_import(&mut production, 10);
    assert_eq!(
        target
            .wait_for_revisions(0, 10, Duration::ZERO)
            .expect("wait after cancel"),
        RevisionWaitOutcome::Cancelled
    );
    assert_eq!(
        sequences(
            &unaffected
                .wait_for_revisions(0, 10, Duration::ZERO)
                .expect("other waiter")
        ),
        [1]
    );
}

#[test]
fn waiter_rejects_a_replaced_production_file() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("waits.pproj");
    let replacement = directory.path().join("replacement.pproj");
    let production = SqliteProduction::create(&path, None).expect("create");
    drop(SqliteProduction::create(&replacement, None).expect("create replacement"));
    fs::copy(&replacement, &path).expect("replace production file");
    assert_eq!(
        production
            .revision_waiter()
            .expect_err("replaced file")
            .kind(),
        ErrorKind::Conflict
    );
}

#[test]
fn commit_worker_process() {
    let Some(production_path) = env::var_os(COMMIT_WORKER_PRODUCTION) else {
        return;
    };
    let mut signal = [0_u8];
    std::io::stdin()
        .read_exact(&mut signal)
        .expect("parent starts worker");
    let mut production = SqliteProduction::open(Path::new(&production_path)).expect("open");
    commit_import(&mut production, 40);
}

#[test]
fn commits_from_another_process_are_observed() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("waits.pproj");
    let production = SqliteProduction::create(&path, None).expect("create");
    let mut waiter = production.revision_waiter().expect("create waiter");
    assert_eq!(
        waiter
            .wait_for_revisions(0, 10, Duration::ZERO)
            .expect("record empty journal"),
        RevisionWaitOutcome::TimedOut
    );

    let mut child = Command::new(env::current_exe().expect("current test executable"))
        .args(["--exact", "commit_worker_process", "--nocapture"])
        .env(COMMIT_WORKER_PRODUCTION, &path)
        .stdin(Stdio::piped())
        .spawn()
        .expect("spawn commit worker");
    child
        .stdin
        .take()
        .expect("worker stdin")
        .write_all(b"x")
        .expect("start commit worker");
    let outcome = waiter
        .wait_for_revisions(0, 10, MAX_REVISION_WAIT)
        .expect("wait for worker commit");
    assert!(child.wait().expect("wait for worker").success());
    assert_eq!(sequences(&outcome), [1]);
}
