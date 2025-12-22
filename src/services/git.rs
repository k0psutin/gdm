use anyhow::Result;
use anyhow::{Context, bail};
use gix::bstr::BString;
use gix::bstr::ByteSlice;
use gix::object::{Kind, tree};
use gix::remote;
use std::fs;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use crate::config::{AppConfig, DefaultAppConfig};

#[derive(Default)]
pub struct DefaultGitService {
    pub app_config: DefaultAppConfig,
}

#[cfg_attr(test, mockall::automock)]
pub trait GitService: Send + Sync + 'static {
    fn shallow_fetch_repository(
        &self,
        repo_url: &str,
        repo_ref: Option<String>,
    ) -> Result<(PathBuf, usize)>;
    fn extract_tree<'a>(
        &self,
        repo: &gix::Repository,
        tree: &gix::Tree<'a>,
        root: &Path,
        file_count: &mut usize,
    ) -> Result<()>;
    fn extract_repo_name_from_src(&self, src: &Path) -> Result<String>;
}

#[cfg_attr(test, mockall::automock)]
impl GitService for DefaultGitService {
    fn shallow_fetch_repository(
        &self,
        repo_url: &str,
        repo_ref: Option<String>,
    ) -> Result<(PathBuf, usize)> {
        let target_ref = repo_ref.unwrap_or("main".into());
        let cache_folder = self.app_config.get_cache_folder_path();
        let addon_folder = self.app_config.get_addon_folder_path();

        let url = gix::url::parse(repo_url.into())?;
        let repo_name = url.path.to_path().unwrap().file_stem().unwrap();
        let dst = cache_folder.join(repo_name);

        if dst.exists() {
            fs::remove_dir_all(&dst)?;
        }
        fs::create_dir_all(&dst)?;

        let mut repo = gix::init(&dst)?;

        // Set a generic fallback committer to avoid errors when no user identity is configured
        // This is required by gitoxide when updating references during fetch operations
        repo.committer_or_set_generic_fallback()?;

        let mut remote = repo.remote_at(url)?;

        remote.replace_refspecs(
            std::iter::once(BString::from(format!("{}:{}", target_ref, target_ref))),
            remote::Direction::Fetch,
        )?;

        let connection = remote.connect(remote::Direction::Fetch)?;
        let prepare_fetch = connection
            .prepare_fetch(gix::progress::Discard, remote::ref_map::Options::default())?;

        let _outcome = prepare_fetch
            .with_shallow(remote::fetch::Shallow::DepthAtRemote(
                NonZeroU32::new(1).unwrap(),
            ))
            .receive(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;

        let mut reference = repo.find_reference(&target_ref)?;
        let commit = reference.peel_to_commit()?;
        let tree = commit.tree()?;
        let dst_addons_path = dst.join("addons");
        let mut file_count = 0;
        if let Some(addons_entry) = tree.find_entry(addon_folder.to_str().unwrap()) {
            let addons_tree = repo.find_object(addons_entry.oid())?.into_tree();
            self.extract_tree(&repo, &addons_tree, &dst_addons_path, &mut file_count)?;
        } else {
            bail!(format!(
                "Warning: No '{:?}' folder found in this commit.",
                addon_folder
            ));
        }

        Ok((dst, file_count))
    }

    fn extract_tree<'a>(
        &self,
        repo: &gix::Repository,
        tree: &'a gix::Tree<'a>,
        root: &Path,
        file_count: &mut usize,
    ) -> Result<()> {
        fs::create_dir_all(root)?;

        for entry in tree.iter() {
            let entry = entry?;
            let path = root.join(entry.filename().to_str_lossy().as_ref());

            match entry.kind() {
                tree::EntryKind::Blob | tree::EntryKind::BlobExecutable => {
                    let object = repo.find_object(entry.oid())?;
                    let blob = object.peel_to_kind(Kind::Blob)?;
                    fs::write(&path, blob.data.as_slice())?;
                    *file_count += 1;
                }
                tree::EntryKind::Tree => {
                    let object = repo.find_object(entry.oid())?;
                    let subtree = object.into_tree();
                    self.extract_tree(repo, &subtree, &path, file_count)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Extracts the repository name from the cache path.
    /// Assumes the path structure is `.../cache_folder/repo_name`.
    fn extract_repo_name_from_src(&self, src: &Path) -> Result<String> {
        src.iter()
            .nth(1)
            .context("No main plugin folder found in path")?
            .to_str()
            .map(|s| s.to_string())
            .context("Failed to convert main plugin folder to string")
    }
}
