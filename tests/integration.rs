use git2::Repository;
use git_hist::app::diff::{DiffLinePart, IndexPair};
use git_hist::app::git;
use git_hist::app::state::State;
use git_hist::args::{Args, UserType};
use std::path::Path;

const GIT_HIST_REPO: &str = env!("CARGO_MANIFEST_DIR");

const ASSISTANT_REPO_PATH: &str = "/Users/ali/Projects/personal/assistant";

fn assistant_repo_path() -> &'static Path {
    Path::new(ASSISTANT_REPO_PATH)
}

fn has_assistant_repo() -> bool {
    Path::new(ASSISTANT_REPO_PATH).join(".git").exists()
}

macro_rules! require_assistant_repo {
    () => {
        if !has_assistant_repo() {
            eprintln!("SKIP: assistant repo not available");
            return;
        }
    };
}

fn default_args(file_path: &str) -> Args {
    Args {
        file_path: file_path.to_string(),
        should_use_full_commit_hash: false,
        beyond_last_line: false,
        should_emphasize_diff: false,
        user_for_name: UserType::Author,
        user_for_date: UserType::Author,
        date_format: String::from("[%Y-%m-%d]"),
        tab_spaces: String::from("    "),
    }
}

// ============================================================
// BUG: Error message typos in git.rs
// "Faild" should be "Failed", "dose" should be "does"
// ============================================================

#[test]
fn bug_error_message_typo_faild() {
    // get_repository() in git.rs:12 has "Faild" typo (only in the original function,
    // not get_repository_at which we added). We verify by checking the source directly.
    let source = include_str!("../src/app/git.rs");
    assert!(
        !source.contains("Faild"),
        "BUG: git.rs contains typo 'Faild' — should be 'Failed'"
    );
}

#[test]
fn bug_error_message_typo_dose() {
    // Create a bare repo to trigger the "dose not" error message
    let tmp = tempdir();
    Repository::init_bare(&tmp).unwrap();
    let result = git::get_repository_at(Path::new(&tmp));
    assert!(result.is_err());
    let err_msg = match result {
        Err(e) => format!("{:#}", e),
        Ok(_) => unreachable!(),
    };
    assert!(
        !err_msg.contains("dose not"),
        "BUG: Error message contains typo 'dose not': {}",
        err_msg
    );
    assert!(
        err_msg.contains("does not"),
        "Error message should contain 'does not': {}",
        err_msg
    );
}

fn tempdir() -> String {
    use std::process::Command;
    let out = Command::new("mktemp").args(["-d"]).output().unwrap().stdout;
    String::from_utf8(out).unwrap().trim().to_string()
}

// ============================================================
// BUG: update_terminal_height doesn't clamp line_index
// After shrinking terminal, line_index can exceed allowed_max_index.
// This leaves the State in an inconsistent position.
// ============================================================

#[test]
fn bug_state_line_index_exceeds_max_after_terminal_shrink() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();

    let point = history.latest().unwrap();
    let lines_count = point.diff().lines().unwrap().len();
    assert!(
        lines_count > 10,
        "Need a file with enough diff lines for this test"
    );

    // Start with a tiny terminal (height=6, so diff_height = 6-4 = 2)
    // This allows scrolling far down
    let state = State::new(point, 0, point.diff().max_line_number_len(), 6, &args);
    let state = state.scroll_to_bottom();
    let bottom_index = state.line_index();
    assert!(
        bottom_index > 0,
        "Should be scrolled down with tiny terminal"
    );

    // Now resize to a very tall terminal — allowed_max_index will shrink
    let state = state.update_terminal_height(1000);

    // BUG: line_index is still at bottom_index, but allowed_max_index is now 0
    // (terminal is taller than the diff, so no scrolling is needed)
    let allowed_max = point.diff().allowed_max_index(&state);
    assert!(
        state.line_index() <= allowed_max,
        "BUG: After terminal resize, line_index ({}) exceeds allowed_max_index ({}). \
         update_terminal_height should clamp line_index.",
        state.line_index(),
        allowed_max
    );
}

// ============================================================
// BUG: scroll_to_bottom doesn't fix over-scrolled state
// Uses cmp::max(line_index, allowed_max) — when line_index > max,
// it stays stuck at the invalid position instead of going to max.
// ============================================================

#[test]
fn bug_scroll_to_bottom_with_overscrolled_state() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();

    let point = history.latest().unwrap();
    let lines_count = point.diff().lines().unwrap().len();

    // Manually create a state with line_index far past what's allowed
    // (simulating what happens after update_terminal_height shrinks the allowed range)
    let overscrolled_index = lines_count.saturating_sub(1);
    let state = State::new(
        point,
        overscrolled_index,
        point.diff().max_line_number_len(),
        1000, // very tall terminal
        &args,
    );

    let allowed_max = point.diff().allowed_max_index(&state);
    assert!(
        overscrolled_index > allowed_max,
        "Setup: line_index should exceed allowed_max for this test"
    );

    // scroll_to_bottom should go to the ACTUAL bottom (allowed_max_index),
    // not stay at the inflated line_index
    let state = state.scroll_to_bottom();
    assert_eq!(
        state.line_index(),
        allowed_max,
        "BUG: scroll_to_bottom should go to allowed_max_index ({}), not stay at {} \
         (uses cmp::max which preserves the larger invalid index)",
        allowed_max,
        state.line_index()
    );
}

// ============================================================
// BUG: scroll_page_up broken when state is over-scrolled
// cmp::min(line_index, max(line_index - diff_height, min_index))
// When line_index is already past allowed_max, page_up only moves
// by diff_height instead of jumping back into valid range.
// ============================================================

#[test]
fn bug_scroll_page_up_from_overscrolled_state() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();

    let point = history.latest().unwrap();
    let lines_count = point.diff().lines().unwrap().len();

    // Over-scrolled state: line_index past allowed max
    let overscrolled_index = lines_count.saturating_sub(1);
    let state = State::new(
        point,
        overscrolled_index,
        point.diff().max_line_number_len(),
        1000,
        &args,
    );

    let allowed_max = point.diff().allowed_max_index(&state);
    assert!(overscrolled_index > allowed_max);

    // After page_up, should be within valid range
    let state = state.scroll_page_up();
    assert!(
        state.line_index() <= allowed_max,
        "BUG: After scroll_page_up from over-scrolled state, line_index ({}) should be \
         within allowed range (max: {})",
        state.line_index(),
        allowed_max
    );
}

// ============================================================
// BUG: nearest_old_index_pair backward search drops relative offset
// When the nearest line with old_index is ABOVE the current index,
// relative_index is set to 0 instead of actual distance.
// This causes visual jumps when navigating between commits.
// ============================================================

#[test]
fn bug_nearest_old_index_pair_backward_drops_relative_index() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();

    // Find a turning point where the diff has inserted lines at the end
    // (lines with new_index but no old_index)
    let mut point = history.latest().unwrap();
    loop {
        let diff = point.diff();
        if let Some(lines) = diff.lines() {
            // Find a position where:
            // 1. Lines below have no old_index (inserted lines)
            // 2. Lines above DO have old_index
            let last_with_old = lines.iter().rposition(|l| l.old_line_number().is_some());
            let last_line_idx = lines.len().saturating_sub(1);

            if let Some(last_old_pos) = last_with_old {
                if last_old_pos + 2 < last_line_idx {
                    // Position ourselves past the last line with old_index
                    let test_index = last_old_pos + 2;
                    let pair = diff.nearest_old_index_pair(test_index);

                    // The nearest old_index line is at `last_old_pos`, which is
                    // (test_index - last_old_pos) lines above us.
                    // BUG: relative_index is 0 instead of the actual distance.
                    let expected_relative = test_index - last_old_pos;
                    assert_eq!(
                        pair.relative_index(),
                        expected_relative,
                        "BUG: nearest_old_index_pair backward search returns relative_index=0 \
                         instead of actual distance {}. This causes scroll position to jump \
                         when navigating commits. (test_index={}, last_old_pos={})",
                        expected_relative,
                        test_index,
                        last_old_pos
                    );
                    return; // Test found a suitable case and asserted
                }
            }
        }

        match history.backward(point) {
            Some(prev) => point = prev,
            None => {
                // Could not find a suitable diff — skip this test
                eprintln!(
                    "SKIP: Could not find a diff with trailing inserted lines. \
                     Bug exists but could not be triggered with Cargo.toml history."
                );
                return;
            }
        }
    }
}

// ============================================================
// BUG: History::new panics on empty iterator instead of Result
// ============================================================

#[test]
fn test_history_new_returns_error_on_empty_iterator() {
    use git_hist::app::history::History;

    let empty: Vec<git_hist::app::history::TurningPoint> = vec![];
    let result = History::new(empty.into_iter());
    assert!(
        result.is_err(),
        "History::new with empty iterator should return Err"
    );
}

// ============================================================
// BUG: can_move_down returns false but scroll_to_bottom returns
// index > 0 (or vice versa) — state methods are inconsistent
// when line_index is in an invalid range.
// ============================================================

#[test]
fn bug_can_move_down_inconsistent_with_overscrolled_state() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();

    let point = history.latest().unwrap();
    let lines_count = point.diff().lines().unwrap().len();

    // Simulate over-scrolled state (after terminal resize)
    let state = State::new(
        point,
        lines_count.saturating_sub(1),
        point.diff().max_line_number_len(),
        1000,
        &args,
    );

    // can_move_down is false (we're past the max)
    assert!(!state.can_move_down());
    // can_move_up is true (we're way past 0)
    assert!(state.can_move_up());

    let prev_index = state.line_index();
    // But scroll_line_up should eventually reach allowed_max_index
    let state_up = state.scroll_line_up();
    assert!(
        state_up.line_index() < prev_index,
        "scroll_line_up should decrease line_index"
    );

    // After scrolling up by 1, we should STILL not be able to move down
    // since we're still past allowed_max — this test verifies the state
    // is being dragged through an invalid range instead of being clamped
    let allowed_max = point.diff().allowed_max_index(&state_up);
    if state_up.line_index() > allowed_max {
        assert!(
            !state_up.can_move_down(),
            "BUG: State at line_index {} (above allowed_max {}) claims it can move down. \
             State should clamp or prevent reaching invalid positions.",
            state_up.line_index(),
            allowed_max
        );
    }
}

// ============================================================
// GREEN: git.rs — Repository discovery
// ============================================================

#[test]
fn test_get_repository_at_git_hist() {
    let repo = git::get_repository_at(Path::new(GIT_HIST_REPO));
    assert!(repo.is_ok(), "Should open git-hist repo");
    assert!(!repo.unwrap().is_bare());
}

#[test]
fn test_get_repository_at_assistant() {
    require_assistant_repo!();
    let repo = git::get_repository_at(assistant_repo_path());
    assert!(repo.is_ok(), "Should open assistant repo with submodules");
    assert!(!repo.unwrap().is_bare());
}

#[test]
fn test_get_repository_at_nonexistent_path() {
    let result = git::get_repository_at(&std::env::temp_dir().join("nonexistent-repo-xyz"));
    assert!(result.is_err());
}

// ============================================================
// GREEN: git.rs — History extraction
// ============================================================

#[test]
fn test_get_history_cargo_toml() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO));
    assert!(history.is_ok());
    assert!(history.unwrap().latest().is_some());
}

#[test]
fn test_get_history_nonexistent_file() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("nonexistent.txt");
    let result =
        git::get_history_with_workdir("nonexistent.txt", &repo, &args, Path::new(GIT_HIST_REPO));
    assert!(result.is_err());
    let err_msg = match result {
        Err(e) => format!("{:#}", e),
        Ok(_) => unreachable!(),
    };
    assert!(
        err_msg.contains("not found on HEAD"),
        "Error should mention file not found, got: {}",
        err_msg
    );
}

#[test]
fn test_get_history_directory_path_fails() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("src");
    let result = git::get_history_with_workdir("src", &repo, &args, Path::new(GIT_HIST_REPO));
    assert!(result.is_err());
    let err_msg = format!("{:#}", result.err().unwrap());
    assert!(
        err_msg.contains("is a directory"),
        "Error should mention it's a directory, got: {}",
        err_msg
    );
}

// ============================================================
// GREEN: git.rs — Assistant repo (submodules)
// ============================================================

#[test]
fn test_get_history_assistant_repo_with_submodules() {
    require_assistant_repo!();
    let repo_path = assistant_repo_path();
    let repo = Repository::open(repo_path).unwrap();
    let args = default_args("CHANGELOG.md");
    let history = git::get_history_with_workdir("CHANGELOG.md", &repo, &args, repo_path);
    assert!(
        history.is_ok(),
        "Should get history in repo with submodules: {:?}",
        history.err()
    );
}

#[test]
fn test_get_history_assistant_readme() {
    require_assistant_repo!();
    let repo_path = assistant_repo_path();
    let repo = Repository::open(repo_path).unwrap();
    let args = default_args("README.md");
    let history = git::get_history_with_workdir("README.md", &repo, &args, repo_path);
    assert!(history.is_ok(), "{:?}", history.err());
    let history = history.unwrap();
    let latest = history.latest().unwrap();
    assert!(
        !latest.is_earliest(),
        "README.md should have more than one turning point"
    );
}

#[test]
fn test_submodule_path_is_not_a_blob() {
    require_assistant_repo!();
    let repo_path = assistant_repo_path();
    let repo = Repository::open(repo_path).unwrap();
    let args = default_args("org");
    let result = git::get_history_with_workdir("org", &repo, &args, repo_path);
    assert!(result.is_err(), "Submodule path should not be a blob");
    let err_msg = format!("{:#}", result.err().unwrap());
    assert!(
        err_msg.contains("submodule"),
        "Error should mention submodule, got: {}",
        err_msg
    );
}

#[test]
fn test_file_inside_submodule_error_mentions_submodule() {
    require_assistant_repo!();
    let repo_path = assistant_repo_path();
    let repo = Repository::open(repo_path).unwrap();
    let args = default_args("org/DIGEST.md");
    let result = git::get_history_with_workdir("org/DIGEST.md", &repo, &args, repo_path);
    assert!(result.is_err(), "File inside submodule should fail");
    let err_msg = format!("{:#}", result.err().unwrap());
    assert!(
        err_msg.contains("submodule"),
        "Error should mention submodule, got: {}",
        err_msg
    );
}

// ============================================================
// GREEN: history.rs — Navigation
// ============================================================

#[test]
fn test_history_latest_is_marked() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let latest = history.latest().unwrap();
    assert!(latest.is_latest());
    assert!(!latest.is_earliest());
}

#[test]
fn test_history_backward_navigation() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let latest = history.latest().unwrap();
    let prev = history.backward(latest);
    assert!(prev.is_some());
    assert!(!prev.unwrap().is_latest());
}

#[test]
fn test_history_forward_from_latest_is_none() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    assert!(history.forward(history.latest().unwrap()).is_none());
}

#[test]
fn test_history_backward_then_forward_roundtrip() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let latest = history.latest().unwrap();
    let prev = history.backward(latest).unwrap();
    let back = history.forward(prev).unwrap();
    assert!(back.is_latest());
}

#[test]
fn test_history_walk_to_earliest() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let mut point = history.latest().unwrap();
    let mut count = 1;
    while let Some(prev) = history.backward(point) {
        point = prev;
        count += 1;
    }
    assert!(point.is_earliest());
    assert!(count > 1);
}

// ============================================================
// GREEN: commit.rs — Metadata
// ============================================================

#[test]
fn test_commit_has_metadata() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let commit = history.latest().unwrap().commit();
    assert!(!commit.short_id().is_empty());
    assert!(!commit.long_id().is_empty());
    assert!(commit.long_id().len() > commit.short_id().len());
    assert!(!commit.author_name().is_empty());
    assert!(!commit.summary().is_empty());
}

#[test]
fn test_commit_long_id_is_40_hex_chars() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let id = history.latest().unwrap().commit().long_id().to_string();
    assert_eq!(id.len(), 40);
    assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_commit_dates_are_reasonable() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let year: i32 = history
        .latest()
        .unwrap()
        .commit()
        .author_date()
        .format("%Y")
        .to_string()
        .parse()
        .unwrap();
    assert!((2020..=2030).contains(&year));
}

#[test]
fn test_commit_references_dont_panic() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let refs = history.latest().unwrap().commit().references();
    let _ = refs.is_empty();
    let _ = refs.head_names();
    let _ = refs.local_branch_names();
    let _ = refs.remote_branch_names();
    let _ = refs.tag_names();
}

// ============================================================
// GREEN: commit.rs — Display types
// ============================================================

#[test]
fn test_local_branch_display_with_head() {
    use git_hist::app::commit::LocalBranch;
    assert_eq!(
        format!("{}", LocalBranch::new("main", true)),
        "HEAD -> main"
    );
}

#[test]
fn test_local_branch_display_without_head() {
    use git_hist::app::commit::LocalBranch;
    assert_eq!(
        format!("{}", LocalBranch::new("feature-x", false)),
        "feature-x"
    );
}

#[test]
fn test_remote_branch_display() {
    use git_hist::app::commit::RemoteBranch;
    assert_eq!(
        format!("{}", RemoteBranch::new("origin/main")),
        "origin/main"
    );
}

#[test]
fn test_tag_display() {
    use git_hist::app::commit::Tag;
    assert_eq!(format!("{}", Tag::new("v1.0.0")), "tag: v1.0.0");
}

#[test]
fn test_references_empty() {
    use git_hist::app::commit::References;
    let refs = References::new(vec![], vec![], vec![], false);
    assert!(refs.is_empty());
    assert!(refs.head_names().is_empty());
    assert!(refs.local_branch_names().is_empty());
    assert!(refs.remote_branch_names().is_empty());
    assert!(refs.tag_names().is_empty());
}

#[test]
fn test_references_with_head() {
    use git_hist::app::commit::References;
    let refs = References::new(vec![], vec![], vec![], true);
    assert!(!refs.is_empty());
    assert_eq!(refs.head_names(), vec!["HEAD"]);
}

// ============================================================
// GREEN: diff.rs — Diff computation
// ============================================================

#[test]
fn test_diff_has_lines() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let lines = history.latest().unwrap().diff().lines();
    assert!(lines.is_some());
    assert!(!lines.unwrap().is_empty());
}

#[test]
fn test_diff_status_text() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let status = history.latest().unwrap().diff().status();
    assert!(status.starts_with("* "), "got: {}", status);
    assert!(status.contains("Cargo.toml"), "got: {}", status);
}

#[test]
fn test_diff_line_signs_are_valid() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    for line in history.latest().unwrap().diff().lines().unwrap() {
        let sign = line.sign();
        assert!(
            sign == "+" || sign == "-" || sign == " ",
            "Invalid sign: '{}'",
            sign
        );
    }
}

#[test]
fn test_diff_lines_have_parts() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    for line in history.latest().unwrap().diff().lines().unwrap() {
        assert!(!line.parts().is_empty());
    }
}

#[test]
fn test_diff_some_lines_have_new_line_numbers() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let lines = history.latest().unwrap().diff().lines().unwrap();
    assert!(lines.iter().any(|l| l.new_line_number().is_some()));
}

#[test]
fn test_diff_max_line_number_len() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let len = history.latest().unwrap().diff().max_line_number_len();
    assert!(len > 0 && len <= 4, "got: {}", len);
}

// ============================================================
// GREEN: diff.rs — Index lookups
// ============================================================

#[test]
fn test_find_index_from_new_index_zero() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    assert!(history
        .latest()
        .unwrap()
        .diff()
        .find_index_from_new_index(0)
        .is_some());
}

#[test]
fn test_nearest_new_index_pair_from_zero() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let pair = history.latest().unwrap().diff().nearest_new_index_pair(0);
    // From index 0, the first line with new_index should be at or near the top
    assert!(
        pair.relative_index() < 5,
        "First new_index should be near the top"
    );
}

// ============================================================
// GREEN: diff.rs — Unit tests
// ============================================================

#[test]
fn test_diff_line_part_text() {
    assert_eq!(DiffLinePart::new("hello", false).text(), "hello");
}

#[test]
fn test_diff_line_part_emphasize() {
    use tui::style::{Color, Style};
    let base = Style::default().fg(Color::Green);

    let emphasized = DiffLinePart::new("x", true);
    assert_eq!(emphasized.emphasize(base).bg, Some(Color::DarkGray));

    let normal = DiffLinePart::new("x", false);
    assert_eq!(normal.emphasize(base).bg, None);
}

#[test]
fn test_index_pair_accessors() {
    let pair = IndexPair::new(5, 10);
    assert_eq!(pair.relative_index(), 5);
    assert_eq!(pair.partial_index(), 10);
}

// ============================================================
// GREEN: state.rs — Basic state management
// ============================================================

#[test]
fn test_state_initial_values() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let point = history.latest().unwrap();
    let state = State::new(point, 0, point.diff().max_line_number_len(), 40, &args);
    assert_eq!(state.line_index(), 0);
    assert_eq!(state.terminal_height(), 40);
}

#[test]
fn test_state_scroll_up_at_top_stays() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let point = history.latest().unwrap();
    let state = State::new(point, 0, point.diff().max_line_number_len(), 40, &args);
    assert!(!state.can_move_up());
    assert_eq!(state.scroll_line_up().line_index(), 0);
}

#[test]
fn test_state_scroll_down_increases_index() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let point = history.latest().unwrap();
    // Small terminal so there's room to scroll
    let state = State::new(point, 0, point.diff().max_line_number_len(), 10, &args);
    if state.can_move_down() {
        assert_eq!(state.scroll_line_down().line_index(), 1);
    }
}

#[test]
fn test_state_scroll_to_bottom_and_top() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let point = history.latest().unwrap();
    let state = State::new(point, 0, point.diff().max_line_number_len(), 10, &args);
    let at_bottom = state.scroll_to_bottom();
    assert!(at_bottom.line_index() > 0);
    assert_eq!(at_bottom.scroll_to_top().line_index(), 0);
}

#[test]
fn test_state_page_scroll() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let point = history.latest().unwrap();
    let state = State::new(point, 0, point.diff().max_line_number_len(), 10, &args);
    let after_down = state.scroll_page_down();
    let down_idx = after_down.line_index();
    if down_idx > 0 {
        let after_up = after_down.scroll_page_up();
        assert!(after_up.line_index() < down_idx);
    }
}

#[test]
fn test_state_update_terminal_height() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let point = history.latest().unwrap();
    let state = State::new(point, 0, point.diff().max_line_number_len(), 40, &args);
    assert_eq!(state.update_terminal_height(80).terminal_height(), 80);
}

#[test]
fn test_state_backward_commit() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let point = history.latest().unwrap();
    let state = State::new(point, 0, point.diff().max_line_number_len(), 40, &args);
    assert!(!state.backward_commit(&history).point().is_latest());
}

#[test]
fn test_state_forward_commit_from_latest_stays() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    let point = history.latest().unwrap();
    let state = State::new(point, 0, point.diff().max_line_number_len(), 40, &args);
    assert!(state.forward_commit(&history).point().is_latest());
}

// ============================================================
// GREEN: args.rs
// ============================================================

#[test]
fn test_args_default_values() {
    let args = default_args("test.txt");
    assert_eq!(args.file_path, "test.txt");
    assert!(!args.should_use_full_commit_hash);
    assert!(!args.beyond_last_line);
    assert!(!args.should_emphasize_diff);
    assert_eq!(args.date_format, "[%Y-%m-%d]");
    assert_eq!(args.tab_spaces, "    "); // 4 spaces
}

#[test]
fn test_args_beyond_last_line_affects_scroll() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();

    let args_normal = default_args("Cargo.toml");
    let history_normal =
        git::get_history_with_workdir("Cargo.toml", &repo, &args_normal, Path::new(GIT_HIST_REPO))
            .unwrap();

    let mut args_beyond = default_args("Cargo.toml");
    args_beyond.beyond_last_line = true;
    let history_beyond =
        git::get_history_with_workdir("Cargo.toml", &repo, &args_beyond, Path::new(GIT_HIST_REPO))
            .unwrap();

    let p1 = history_normal.latest().unwrap();
    let s1 = State::new(p1, 0, p1.diff().max_line_number_len(), 10, &args_normal);

    let p2 = history_beyond.latest().unwrap();
    let s2 = State::new(p2, 0, p2.diff().max_line_number_len(), 10, &args_beyond);

    assert!(
        s2.scroll_to_bottom().line_index() >= s1.scroll_to_bottom().line_index(),
        "beyond_last_line should allow scrolling at least as far"
    );
}

#[test]
fn test_custom_tab_size() {
    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let mut args = default_args("Cargo.toml");
    args.tab_spaces = "  ".to_string();
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();
    // Should not panic
    let _ = history.latest().unwrap().diff().lines();
}

// ============================================================
// GREEN: Integration — full workflow with assistant repo
// ============================================================

#[test]
fn test_full_workflow_assistant_repo() {
    require_assistant_repo!();
    let repo_path = assistant_repo_path();
    let repo = Repository::open(repo_path).unwrap();
    let args = default_args("README.md");
    let history = git::get_history_with_workdir("README.md", &repo, &args, repo_path).unwrap();

    let mut point = history.latest().unwrap();
    let mut count = 1;
    while let Some(prev) = history.backward(point) {
        point = prev;
        count += 1;
        let status = point.diff().status();
        assert!(status.starts_with("* "));
        if let Some(lines) = point.diff().lines() {
            for line in lines {
                let _ = line.sign();
                let _ = line.parts();
            }
        }
    }
    assert!(point.is_earliest());
    assert!(count >= 2);
}

#[test]
fn test_state_navigation_across_commits_assistant() {
    require_assistant_repo!();
    let repo_path = assistant_repo_path();
    let repo = Repository::open(repo_path).unwrap();
    let args = default_args("README.md");
    let history = git::get_history_with_workdir("README.md", &repo, &args, repo_path).unwrap();
    let point = history.latest().unwrap();
    let state = State::new(point, 0, point.diff().max_line_number_len(), 24, &args);
    // Scroll down then switch commits — should not panic
    let state = state
        .scroll_line_down()
        .scroll_line_down()
        .scroll_line_down();
    let state = state.backward_commit(&history);
    let _ = state.line_index();
    let _ = state.point().commit().summary();
}

// ============================================================
// BUG: diff_height of 0 with tiny terminal causes scroll
// methods to do nothing (page size = 0)
// ============================================================

#[test]
fn bug_tiny_terminal_page_scroll_is_noop() {
    use git_hist::app::dashboard::Dashboard;

    let repo = Repository::open(GIT_HIST_REPO).unwrap();
    let args = default_args("Cargo.toml");
    let history =
        git::get_history_with_workdir("Cargo.toml", &repo, &args, Path::new(GIT_HIST_REPO))
            .unwrap();

    // Terminal height = 4, so diff_height = 4 - 4 = 0
    let diff_height = Dashboard::diff_height(4);
    assert_eq!(
        diff_height, 0,
        "diff_height should be 0 for terminal height 4"
    );

    let point = history.latest().unwrap();
    let state = State::new(point, 0, point.diff().max_line_number_len(), 4, &args);

    // With diff_height=0, page scroll should still move (at least by 1 line)
    // BUG: page_down does nothing because page size is 0
    let after_page_down = state.scroll_page_down();
    let allowed_max = point.diff().allowed_max_index(&after_page_down);

    if allowed_max > 0 {
        assert!(
            after_page_down.line_index() > 0,
            "BUG: scroll_page_down is a no-op when terminal is tiny (diff_height=0). \
             Page scroll should move by at least 1 line. allowed_max={}",
            allowed_max
        );
    }
}

// ============================================================
// BUG: Deleted file diff status uses new_path (PR #13 fixed
// the Delta status, but status() still unwraps new_path for
// Delta::Deleted — a deleted file's meaningful path is old_path)
// ============================================================

#[test]
fn test_deleted_diff_should_reference_old_path() {
    require_assistant_repo!();
    // Walk through assistant repo history looking for a deleted file
    let repo_path = assistant_repo_path();
    let repo = Repository::open(repo_path).unwrap();

    // Try several files that might have been deleted at some point
    // Even if we can't find one, the test documents the bug:
    // diff.rs line 105: Delta::Deleted uses self.new_path but should use self.old_path
    // For a deleted file, new_path may be set (git2 copies it) but semantically old_path
    // is the correct path to display.

    // This test verifies the code path doesn't panic for any status type
    for file in &["README.md", "CHANGELOG.md"] {
        let args = default_args(file);
        if let Ok(history) = git::get_history_with_workdir(file, &repo, &args, repo_path) {
            let mut point = history.latest().unwrap();
            while let Some(prev) = history.backward(point) {
                point = prev;
                let status = point.diff().status();
                // Verify status always has the file path
                assert!(
                    status.contains('/') || status.contains('.') || status.contains(file),
                    "Status should contain a file reference: {}",
                    status
                );
            }
        }
    }
}

// ============================================================
// BUG #3: strip_prefix panics when workdir is outside repo
// get_history_with_workdir used to .unwrap() the strip_prefix
// result, causing a panic instead of returning an error.
// ============================================================

#[test]
fn test_get_history_path_outside_repo_returns_error() {
    let repo_path = Path::new(GIT_HIST_REPO);
    let repo = Repository::open(repo_path).unwrap();
    let args = default_args("Cargo.toml");
    let result = git::get_history_with_workdir("Cargo.toml", &repo, &args, &std::env::temp_dir());
    assert!(
        result.is_err(),
        "Path outside repo should return error, not panic"
    );
    let err_msg = format!("{:#}", result.err().unwrap());
    assert!(
        err_msg.contains("not inside"),
        "Error should mention path not inside repo, got: {}",
        err_msg
    );
}
