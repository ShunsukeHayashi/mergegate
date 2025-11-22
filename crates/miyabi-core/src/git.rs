//! Git utilities for repository discovery and validation
//!
//! Provides utilities for working with Git repositories:
//! - Repository root discovery from any subdirectory
//! - Repository validation
//! - Branch detection

use crate::error::{Error, Result};
use std::path::{Path, PathBuf};

/// Find the Git repository root from the current directory or any subdirectory
///
/// This function uses Git's discovery mechanism to walk up the directory tree
/// until it finds a `.git` directory.
///
/// # Examples
///
/// ```no_run
/// use miyabi_core::git::find_git_root;
///
/// let root = find_git_root(None)?;
/// println!("Git root: {:?}", root);
/// # Ok::<(), miyabi_core::error::Error>(())
/// ```
pub fn find_git_root(start_path: Option<&Path>) -> Result<PathBuf> {
    let search_path = match start_path {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir()?,
    };

    match git2::Repository::discover(&search_path) {
        Ok(repo) => repo
            .workdir()
            .map(|p| p.to_path_buf())
            .ok_or_else(|| {
                Error::Git(
                    "Repository is bare (no working directory). Miyabi requires a non-bare repository.".to_string()
                )
            }),
        Err(e) => Err(Error::Git(format!(
            "Not in a Git repository. Please run miyabi from within a Git repository.\n\
             Searched from: {:?}\n\
             Git error: {}\n\n\
             To initialize a new repository, run: git init",
            search_path, e
        ))),
    }
}

/// Validate that a path is a valid Git repository
///
/// # Arguments
/// * `path` - Path to validate
///
/// # Returns
/// `true` if the path is a valid Git repository with a working directory
pub fn is_valid_repository(path: impl AsRef<Path>) -> bool {
    match git2::Repository::open(path.as_ref()) {
        Ok(repo) => repo.workdir().is_some(),
        Err(_) => false,
    }
}

/// Check if a path is within a Git repository
///
/// # Arguments
/// * `path` - Path to check
///
/// # Returns
/// `true` if the path is within a Git repository
pub fn is_in_git_repo(path: impl AsRef<Path>) -> bool {
    git2::Repository::discover(path.as_ref()).is_ok()
}

/// Get the current branch name of a repository
///
/// # Arguments
/// * `repo_path` - Path to the repository
///
/// # Returns
/// The name of the currently checked out branch
///
/// # Errors
/// Returns an error if the repository is in a detached HEAD state or cannot be opened
pub fn get_current_branch(repo_path: impl AsRef<Path>) -> Result<String> {
    let repo = git2::Repository::open(repo_path.as_ref())
        .map_err(|e| Error::Git(format!("Failed to open repository: {}", e)))?;

    let head = repo
        .head()
        .map_err(|e| Error::Git(format!("Failed to get HEAD: {}", e)))?;

    if !head.is_branch() {
        return Err(Error::Git(
            "Repository is in detached HEAD state. Please checkout a branch.".to_string(),
        ));
    }

    head.shorthand()
        .map(|s| s.to_string())
        .ok_or_else(|| Error::Git("Failed to get branch name".to_string()))
}

/// Get the main branch name (tries 'main' then 'master')
///
/// # Arguments
/// * `repo_path` - Path to the repository
///
/// # Returns
/// The name of the main branch ('main' or 'master')
///
/// # Errors
/// Returns an error if neither 'main' nor 'master' branch exists
pub fn get_main_branch(repo_path: impl AsRef<Path>) -> Result<String> {
    let repo = git2::Repository::open(repo_path.as_ref())
        .map_err(|e| Error::Git(format!("Failed to open repository: {}", e)))?;

    if repo.find_branch("main", git2::BranchType::Local).is_ok() {
        Ok("main".to_string())
    } else if repo.find_branch("master", git2::BranchType::Local).is_ok() {
        Ok("master".to_string())
    } else {
        Err(Error::Git(
            "Neither 'main' nor 'master' branch found. Please create a main branch.".to_string(),
        ))
    }
}

/// Check if the repository has uncommitted changes
///
/// # Arguments
/// * `repo_path` - Path to the repository
///
/// # Returns
/// `true` if there are uncommitted changes in the working directory or index
pub fn has_uncommitted_changes(repo_path: impl AsRef<Path>) -> Result<bool> {
    let repo = git2::Repository::open(repo_path.as_ref())
        .map_err(|e| Error::Git(format!("Failed to open repository: {}", e)))?;

    let statuses = repo
        .statuses(None)
        .map_err(|e| Error::Git(format!("Failed to get repository status: {}", e)))?;

    Ok(!statuses.is_empty())
}

/// Get the short hash of HEAD
pub fn get_head_short_hash(repo_path: impl AsRef<Path>) -> Result<String> {
    let repo = git2::Repository::open(repo_path.as_ref())
        .map_err(|e| Error::Git(format!("Failed to open repository: {}", e)))?;

    let head = repo
        .head()
        .map_err(|e| Error::Git(format!("Failed to get HEAD: {}", e)))?;

    let commit = head
        .peel_to_commit()
        .map_err(|e| Error::Git(format!("Failed to get commit: {}", e)))?;

    Ok(commit.id().to_string()[..7].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf();
        git2::Repository::init(&repo_path).unwrap();
        (temp_dir, repo_path)
    }

    #[test]
    fn test_find_git_root_from_root() {
        let (_temp, repo_path) = setup_test_repo();
        let found_root = find_git_root(Some(&repo_path)).unwrap();
        let canonical_found = fs::canonicalize(&found_root).unwrap();
        let canonical_expected = fs::canonicalize(&repo_path).unwrap();
        assert_eq!(canonical_found, canonical_expected);
    }

    #[test]
    fn test_find_git_root_from_subdirectory() {
        let (_temp, repo_path) = setup_test_repo();
        let sub_dir = repo_path.join("src").join("nested");
        fs::create_dir_all(&sub_dir).unwrap();
        let found_root = find_git_root(Some(&sub_dir)).unwrap();
        let canonical_found = fs::canonicalize(&found_root).unwrap();
        let canonical_expected = fs::canonicalize(&repo_path).unwrap();
        assert_eq!(canonical_found, canonical_expected);
    }

    #[test]
    fn test_is_valid_repository() {
        let (_temp, repo_path) = setup_test_repo();
        assert!(is_valid_repository(&repo_path));
        let temp_dir = TempDir::new().unwrap();
        assert!(!is_valid_repository(temp_dir.path()));
    }

    #[test]
    fn test_is_in_git_repo() {
        let (_temp, repo_path) = setup_test_repo();
        assert!(is_in_git_repo(&repo_path));
        let temp_dir = TempDir::new().unwrap();
        assert!(!is_in_git_repo(temp_dir.path()));
    }
}
