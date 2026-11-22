use crate::SyncEngine;

fn engine() -> (tempfile::TempDir, SyncEngine) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("history.srt");
    // Subtitle ID order deliberately differs from timestamp order.
    std::fs::write(
        &path,
        "1\n00:00:03,000 --> 00:00:03,500\nOne\n\n2\n00:00:01,000 --> 00:00:01,500\nTwo\n\n3\n00:00:02,000 --> 00:00:02,500\nThree\n",
    )
    .unwrap();
    (directory, SyncEngine::new(path).unwrap())
}

#[test]
fn undo_redo_restore_exact_anchors_independent_of_id_and_time_order() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("out-of-order.srt");
    std::fs::write(&path, "1\n00:01:00,000 --> 00:01:01,000\nOne\n\n2\n00:00:10,000 --> 00:00:11,000\nTwo\n\n3\n00:00:30,000 --> 00:00:31,000\nThree\n").unwrap();
    let mut engine = SyncEngine::new(path).unwrap();
    engine.add_anchor(1, 60_250, true).unwrap();
    engine.add_anchor(2, 10_400, true).unwrap();
    engine.add_anchor(3, 30_100, false).unwrap();
    let expected = engine.get_anchors().to_vec();
    assert_eq!(
        expected
            .iter()
            .map(|a| a.subtitle_index)
            .collect::<Vec<_>>(),
        [2, 3, 1]
    );

    assert!(engine.undo());
    assert_eq!(
        engine
            .get_anchors()
            .iter()
            .map(|a| a.subtitle_index)
            .collect::<Vec<_>>(),
        [2, 1]
    );
    assert!(engine.redo());
    assert_eq!(engine.get_anchors(), expected);
}

#[test]
fn editing_existing_anchor_undoes_to_original_value() {
    let (_directory, mut engine) = engine();
    engine.add_anchor(2, 1_200, true).unwrap();
    let original = engine.get_anchors().to_vec();
    engine.add_anchor(2, 1_700, true).unwrap();
    assert_ne!(engine.get_anchors(), original);
    assert!(engine.undo());
    assert_eq!(engine.get_anchors(), original);
    assert!(engine.redo());
    assert_eq!(engine.get_anchors()[0].corrected_time_ms, 1_700);
}

#[test]
fn removing_anchor_undo_restores_anchor_mapping_and_checked_count() {
    let (_directory, mut engine) = engine();
    engine.add_anchor(1, 3_500, true).unwrap();
    engine.add_anchor(2, 1_700, true).unwrap();
    let previous_mapping = engine.get_synced_times(3).unwrap();
    assert_eq!(engine.checked_count(), 2);

    assert!(engine.remove_anchor(1));
    assert_eq!(engine.checked_count(), 1);
    assert_ne!(engine.get_synced_times(3).unwrap(), previous_mapping);
    assert!(engine.undo());
    assert_eq!(engine.checked_count(), 2);
    assert_eq!(engine.get_synced_times(3), Some(previous_mapping));
    assert!(engine.get_anchors().iter().any(|a| a.subtitle_index == 1));
}

#[test]
fn grouped_anchor_edits_are_one_undo_step() {
    let (_directory, mut engine) = engine();
    engine.group_anchor_edits(|engine| {
        engine.add_anchor(1, 3_100, true).unwrap();
        engine.add_anchor(2, 1_100, true).unwrap();
        engine.add_anchor(3, 2_100, true).unwrap();
    });
    assert_eq!(engine.anchor_count(), 3);
    assert_eq!(engine.checked_count(), 3);
    assert!(engine.undo());
    assert_eq!(engine.anchor_count(), 0);
    assert_eq!(engine.checked_count(), 0);
    assert!(!engine.can_undo());
    assert!(engine.redo());
    assert_eq!(engine.anchor_count(), 3);
}

#[test]
fn no_op_and_branching_history_behave_predictably() {
    let (_directory, mut engine) = engine();
    engine.add_anchor(2, 1_200, true).unwrap();
    engine.add_anchor(2, 1_200, true).unwrap();
    assert!(engine.undo());
    assert!(!engine.can_undo());
    assert!(engine.can_redo());
    assert!(!engine.remove_anchor(3));
    assert!(
        engine.can_redo(),
        "a no-op removal must preserve the redo branch"
    );
    engine.add_anchor(3, 2_100, true).unwrap();
    assert!(!engine.can_redo());
    assert!(engine.undo()); // New branch remains undoable.
}

#[test]
fn history_is_bounded_and_reset_starts_fresh() {
    let (_directory, mut engine) = engine();
    for corrected in 3_001..=3_101 {
        engine.add_anchor(1, corrected, true).unwrap();
    }
    for _ in 0..100 {
        assert!(engine.undo());
    }
    assert!(!engine.can_undo());
    assert_eq!(engine.get_anchors()[0].corrected_time_ms, 3_001);

    engine.reset();
    assert!(!engine.can_undo());
    assert!(!engine.can_redo());
    engine.add_anchor(2, 1_100, true).unwrap();
    assert!(engine.can_undo());
}

#[test]
fn loading_a_session_preserves_anchors_but_starts_a_new_history() {
    let (_directory, mut engine) = engine();
    engine.add_anchor(1, 3_100, true).unwrap();
    let state = engine.export_state();
    let restored = SyncEngine::from_state(state).unwrap();
    assert_eq!(restored.get_anchors(), engine.get_anchors());
    assert!(!restored.can_undo());
    assert!(!restored.can_redo());
}
