use crate::app::commit::Commit;
use crate::app::diff::Diff;
use crate::app::history::{History, TurningPoint};
use crate::args::Args;
use anyhow::{anyhow, Context, Result};
use git2::{DiffFindOptions, ObjectType, Repository};
use std::env;
use std::path;

pub fn get_repository() -> Result<Repository> {
    get_repository_at(&env::current_dir()?)
}

pub fn get_repository_at(path: &path::Path) -> Result<Repository> {
    let repo = Repository::discover(path)
        .with_context(|| format!("Failed to open a git repository at '{}'", path.display()))?;
    if repo.is_bare() {
        return Err(anyhow!("git-hist does not support a bare repository"));
    }
    Ok(repo)
}

pub fn get_history<'a, P: AsRef<path::Path>>(
    file_path: P,
    repo: &'a Repository,
    args: &'a Args,
) -> Result<History<'a>> {
    get_history_with_workdir(file_path, repo, args, &env::current_dir().unwrap())
}

pub fn get_history_with_workdir<'a, P: AsRef<path::Path>>(
    file_path: P,
    repo: &'a Repository,
    args: &'a Args,
    workdir: &path::Path,
) -> Result<History<'a>> {
    let repo_root = repo
        .path()
        .parent()
        .context("Failed to determine repository root")?;
    let file_path_from_repository = workdir
        .join(&file_path)
        .strip_prefix(repo_root)
        .with_context(|| {
            format!(
                "File path '{}' is not inside the git repository at '{}'",
                file_path.as_ref().to_string_lossy(),
                repo_root.display()
            )
        })?
        .to_path_buf();

    let mut revwalk = repo
        .revwalk()
        .context("Failed to traverse the commit graph")?;
    revwalk.push_head().context("Failed to find HEAD")?;
    revwalk.simplify_first_parent()?;

    let commits = revwalk
        .filter_map(|oid| oid.and_then(|oid| repo.find_commit(oid)).ok())
        .collect::<Vec<_>>();
    let head_tree = commits
        .first()
        .context("Failed to get any commit")?
        .tree()
        .unwrap();
    let latest_file_oid = head_tree
        .get_path(&file_path_from_repository)
        .map_err(|_| {
            // Check if the path goes through a submodule (commit entry in tree)
            let mut prefix = path::PathBuf::new();
            for component in file_path_from_repository.components() {
                prefix.push(component);
                if let Ok(entry) = head_tree.get_path(&prefix) {
                    if entry.kind() == Some(ObjectType::Commit) {
                        let remaining = file_path_from_repository.strip_prefix(&prefix).unwrap();
                        return anyhow!(
                            "The path '{}' is inside the submodule '{}'. \
                             Run git-hist from within the submodule instead:\n  \
                             cd {} && git hist {}",
                            file_path.as_ref().to_string_lossy(),
                            prefix.display(),
                            prefix.display(),
                            remaining.display()
                        );
                    }
                }
            }
            anyhow!(
                "File '{}' not found on HEAD. Check the path and try again",
                file_path.as_ref().to_string_lossy()
            )
        })
        .and_then(|entry| {
            if let Some(ObjectType::Blob) = entry.kind() {
                Ok(entry)
            } else if entry.kind() == Some(ObjectType::Commit) {
                Err(anyhow!(
                    "The path '{}' is a submodule, not a file. \
                     Run git-hist from within the submodule instead:\n  \
                     cd {}",
                    file_path.as_ref().to_string_lossy(),
                    file_path.as_ref().to_string_lossy()
                ))
            } else {
                Err(anyhow!(
                    "'{}' is a directory, not a file. Provide a path to a file instead",
                    file_path.as_ref().to_string_lossy()
                ))
            }
        })?
        .id();

    let mut file_oid = latest_file_oid;
    let mut file_path = file_path_from_repository;
    let history = History::new(commits.iter().filter_map(|git_commit| {
        let old_tree = git_commit.parent(0).and_then(|p| p.tree()).ok();
        let new_tree = git_commit.tree().ok();
        assert!(new_tree.is_some());

        let mut git_diff = repo
            .diff_tree_to_tree(old_tree.as_ref(), new_tree.as_ref(), None)
            .unwrap();

        // detect file renames
        git_diff
            .find_similar(Some(DiffFindOptions::new().renames(true)))
            .unwrap();

        let delta = git_diff.deltas().find(|delta| {
            delta.new_file().id() == file_oid
                && delta
                    .new_file()
                    .path()
                    .filter(|path| *path == file_path)
                    .is_some()
        });
        if let Some(delta) = delta.as_ref() {
            file_oid = delta.old_file().id();
            file_path = delta.old_file().path().unwrap().to_path_buf();
        }

        delta.map(|delta| {
            let commit = Commit::new(git_commit, repo);
            let diff = Diff::new(&delta, repo, args);
            TurningPoint::new(commit, diff)
        })
    }))?;

    Ok(history)
}
