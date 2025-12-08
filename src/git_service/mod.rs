use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use gix::bstr::BString;
use gix::bstr::ByteSlice;
use gix::object::{Kind, tree};
use gix::remote;
use indicatif::ProgressBar;
use std::fs;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use crate::app_config::AppConfig;
use crate::app_config::DefaultAppConfig;
use crate::plugin_config_repository::plugin::Plugin;
use crate::plugin_config_repository::plugin::PluginSource;

#[derive(Default)]
pub struct DefaultGitService {
    pub app_config: DefaultAppConfig,
}

pub trait GitService: Send + Sync + 'static {
    fn shallow_fetch_repository(
        &self,
        repo_url: &str,
        repo_ref: Option<String>,
    ) -> Result<(PathBuf, usize)>;
    fn move_downloaded_addons(&self, src: &Path, pb_task: ProgressBar) -> Result<Vec<PathBuf>>;
    fn create_plugins_from_addons_paths(
        &self,
        plugin_source: &PluginSource,
        addon_folders: &[PathBuf],
    ) -> Result<Vec<(PathBuf, Plugin)>>;
    fn extract_tree(
        &self,
        repo: &gix::Repository,
        tree: &gix::Tree,
        root: &Path,
        file_count: &mut usize,
    ) -> Result<()>;
    fn find_plugin_cfg_file_greedy(&self, dir: &Path) -> Result<Option<PathBuf>>;
    fn extract_main_plugin_name_from_src(&self, src: &Path) -> Result<String>;
    fn determine_main_plugin_from_main_plugin_name_and_plugins(
        &self,
        plugins: &[(PathBuf, Plugin)],
        main_plugin_name: &str,
    ) -> Result<Plugin>;
    fn add_sub_assets_to_plugin(
        &self,
        plugin: &Plugin,
        plugins: &[(PathBuf, Plugin)],
        addon_folders: &[PathBuf],
    ) -> Result<Plugin>;
}

#[cfg_attr(test, mockall::automock)]
impl GitService for DefaultGitService {
    fn find_plugin_cfg_file_greedy(&self, dir: &Path) -> Result<Option<PathBuf>> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = self.find_plugin_cfg_file_greedy(&path)? {
                    return Ok(Some(found));
                }
            } else if entry.file_name() == "plugin.cfg" {
                return Ok(Some(path));
            }
        }
        Ok(None)
    }

    fn create_plugins_from_addons_paths(
        &self,
        plugin_source: &PluginSource,
        addon_folders: &[PathBuf],
    ) -> Result<Vec<(PathBuf, Plugin)>> {
        let mut plugins: Vec<(PathBuf, Plugin)> = vec![];
        for folder in addon_folders {
            let path = PathBuf::from("addons").join(folder);
            if let Some(plugin_cfg_path) = self.find_plugin_cfg_file_greedy(&path)? {
                plugins.push((
                    folder.clone(),
                    Plugin::from_path(&plugin_cfg_path, plugin_source.clone())?,
                ));
            }
        }
        Ok(plugins)
    }

    fn extract_main_plugin_name_from_src(&self, src: &Path) -> Result<String> {
        src.iter()
            .skip(1)
            .nth(0)
            .context("No main plugin folder found")?
            .to_str()
            .map(|s| s.to_string())
            .context("Failed to convert main plugin folder to string")
    }

    fn determine_main_plugin_from_main_plugin_name_and_plugins(
        &self,
        plugins: &[(PathBuf, Plugin)],
        main_plugin_name: &str,
    ) -> Result<Plugin> {
        let best_match = plugins.iter().fold(
            (PathBuf::new(), Plugin::default(), 0.0),
            |mut best, (path, plugin)| {
                let folder_name = path.to_string_lossy().to_string();
                let jaro_filename = strsim::jaro(
                    &folder_name.to_lowercase(),
                    &main_plugin_name.to_lowercase(),
                );
                let jaro_title =
                    strsim::jaro(&folder_name.to_lowercase(), &plugin.title.to_lowercase());
                let max_jaro = jaro_filename.max(jaro_title);
                if max_jaro > best.2 {
                    best = (PathBuf::from(path), plugin.clone(), max_jaro);
                }
                best
            },
        );
        Ok(best_match.1)
    }

    fn add_sub_assets_to_plugin(
        &self,
        plugin: &Plugin,
        plugins: &[(PathBuf, Plugin)],
        addon_folders: &[PathBuf],
    ) -> Result<Plugin> {
        let main_plugin_folder = plugins
            .iter()
            .find(|(_, p)| p.title == plugin.title)
            .map(|(path, _)| path)
            .context("Main plugin folder not found")?;
        let sub_assets: Vec<String> = addon_folders
            .iter()
            .filter_map(|folder| {
                if folder != main_plugin_folder {
                    Some(folder.to_string_lossy().to_string())
                } else {
                    None
                }
            })
            .collect();

        Ok(Plugin::new(
            plugin.source.clone(),
            plugin.plugin_cfg_path.as_ref().map(PathBuf::from),
            plugin.title.clone(),
            plugin.get_version(),
            plugin.license.clone(),
            sub_assets,
        ))
    }

    fn move_downloaded_addons(&self, src: &Path, pb_task: ProgressBar) -> Result<Vec<PathBuf>> {
        let dir = fs::read_dir(src)?;
        let dst = self.app_config.get_addon_folder_path();
        let dst_root = src.parent().unwrap();

        if !dst.try_exists()? {
            fs::create_dir_all(&dst)?;
        }

        let mut moved_folders = vec![];

        for entry in dir {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let repo_parent = entry.file_name();
            let src_path = src.join(&repo_parent);
            let dst_path = dst.join(&repo_parent);

            if file_type.is_dir() {
                if dst_path.try_exists()? {
                    fs::remove_dir_all(&dst_path)?;
                }
                fs::rename(&src_path, &dst_path)?;
                pb_task.inc(1);
                if let Some(name) = dst_path.file_name() {
                    moved_folders.push(PathBuf::from(name));
                }
            }
        }

        fs::remove_dir_all(dst_root)?;
        Ok(moved_folders)
    }

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

        let repo = gix::init(&dst)?;

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

        Ok((dst_addons_path, file_count))
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
}
