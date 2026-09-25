//! Evaluation of dependency evidence captured on activity inputs.

use postproject_core::{
    Activity, ActivityInput, ArtifactKnowledgeReason, ArtifactKnowledgeState, Result,
};
use rusqlite::params;

use crate::{SqliteProduction, sqlite_error};

pub(crate) struct DependencyEvaluation {
    pub(crate) state: ArtifactKnowledgeState,
    pub(crate) reasons: Vec<ArtifactKnowledgeReason>,
}

pub(crate) fn evaluate_input_dependencies(
    production: &SqliteProduction,
    activity: &Activity,
    input: &ActivityInput,
) -> Result<DependencyEvaluation> {
    let input_id = production
        .connection
        .query_row(
            "SELECT id FROM activity_inputs
             WHERE activity_id = ?1 AND representation_id = ?2 AND role IS ?3",
            params![
                activity.id().as_bytes().as_slice(),
                input.representation_id().as_bytes().as_slice(),
                input.role().map(postproject_core::ActivityRole::as_str),
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(sqlite_error("locate activity input dependency snapshot"))?;
    let snapshot_exists = production
        .connection
        .query_row(
            "SELECT EXISTS (
                SELECT 1 FROM activity_input_dependency_snapshots
                WHERE activity_input_id = ?1
             )",
            [input_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(sqlite_error(
            "read activity input dependency snapshot marker",
        ))?;
    if snapshot_exists {
        return Ok(DependencyEvaluation {
            state: ArtifactKnowledgeState::Current,
            reasons: Vec::new(),
        });
    }
    Ok(DependencyEvaluation {
        state: ArtifactKnowledgeState::Indeterminate,
        reasons: vec![ArtifactKnowledgeReason::DependencySnapshotAbsent {
            activity_id: activity.id(),
            representation_id: input.representation_id(),
        }],
    })
}
