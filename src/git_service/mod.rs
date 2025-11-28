use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use gix::bstr::BString;
use gix::bstr::ByteSlice;
use gix::object::{Kind, tree};
use gix::progress::{Count, Id, NestedProgress, Progress, Unit};
use gix::remote;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::fs;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

use crate::app_config::AppConfig;
use crate::app_config::DefaultAppConfig;
use crate::main;
use crate::plugin_config_repository::plugin::Plugin;
use crate::plugin_config_repository::plugin::PluginSource;

#[derive(Default)]
pub struct DefaultGitService {
    pub app_config: DefaultAppConfig,
}

pub trait GitService: Send + Sync + 'static {
    fn download_plugin(&self, repo_url: &str, repo_ref: Option<&str>) -> Result<(String, Plugin)>;
    fn shallow_fetch_repository(&self, repo_url: &str, repo_ref: Option<&str>) -> Result<PathBuf>;
    fn move_downloaded_addons(&self, src: &PathBuf) -> Result<Vec<PathBuf>>;
    fn create_plugins_from_addons_paths(
        &self,
        plugin_source: &PluginSource,
        addon_folders: &[PathBuf],
    ) -> Result<Vec<(PathBuf, Plugin)>>;
    fn extract_tree(&self, repo: &gix::Repository, tree: &gix::Tree, root: &Path) -> Result<()>;
    fn find_plugin_cfg_file_greedy(&self, dir: &Path) -> Result<Option<PathBuf>>;
    fn extract_main_plugin_name_from_src(&self, src: &PathBuf) -> Result<String>;
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
                println!("entry: {:?}", entry.file_name());
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

    fn extract_main_plugin_name_from_src(&self, src: &PathBuf) -> Result<String> {
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

    fn download_plugin(&self, repo_url: &str, repo_ref: Option<&str>) -> Result<(String, Plugin)> {
        let plugin_source = PluginSource::Git {
            url: repo_url.to_string(),
            reference: repo_ref.unwrap_or("main").to_string(),
        };
        let src = self.shallow_fetch_repository(repo_url, repo_ref)?;
        let main_plugin_name = self.extract_main_plugin_name_from_src(&src)?; // Returns main plugin name e.g. "godot-mod-loader"
        let addon_folders = self.move_downloaded_addons(&src)?; // Returns all addon folder names e.g. ["gut", "mod_loader", "JSON_schema_validator"]
        let plugins = self.create_plugins_from_addons_paths(&plugin_source, &addon_folders)?;
        let plugin = self
            .determine_main_plugin_from_main_plugin_name_and_plugins(&plugins, &main_plugin_name)?;
        let plugin_with_sub_assets =
            self.add_sub_assets_to_plugin(&plugin, &plugins, &addon_folders)?;
        Ok((main_plugin_name, plugin_with_sub_assets))
    }
    fn move_downloaded_addons(&self, src: &PathBuf) -> Result<Vec<PathBuf>> {
        let dir = fs::read_dir(src)?;
        let dst = self.app_config.get_addon_folder_path();
        let dst_root = src.parent().unwrap();
        println!("Moving addons from {:?} to {:?}", src, dst);

        if !dst.try_exists()? {
            println!("Creating addons folder at {:?}", dst);
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
                    println!(
                        "Folder {:?} already exists, removing it first",
                        dst_path.file_name().unwrap_or_default()
                    );
                    fs::remove_dir_all(&dst_path)?;
                }
                fs::rename(&src_path, &dst_path)?;
                if let Some(name) = dst_path.file_name() {
                    moved_folders.push(PathBuf::from(name));
                }
                println!("Moved addon folder from {:?} to {:?}", src_path, dst_path);
            }
        }

        fs::remove_dir_all(dst_root)?;
        Ok(moved_folders)
    }

    fn shallow_fetch_repository(&self, repo_url: &str, repo_ref: Option<&str>) -> Result<PathBuf> {
        let target_ref = repo_ref.unwrap_or("main");
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

        let mut reference = repo.find_reference(target_ref)?;
        let commit = reference.peel_to_commit()?;
        let tree = commit.tree()?;
        let dst_addons_path = dst.join("addons");
        if let Some(addons_entry) = tree.find_entry(addon_folder.to_str().unwrap()) {
            let addons_tree = repo.find_object(addons_entry.oid())?.into_tree();

            self.extract_tree(&repo, &addons_tree, &dst_addons_path)?;
        } else {
            bail!(format!(
                "Warning: No '{:?}' folder found in this commit.",
                addon_folder
            ));
        }

        Ok(dst_addons_path)
    }

    // Recursive function to extract a tree to the filesystem
    fn extract_tree(&self, repo: &gix::Repository, tree: &gix::Tree, root: &Path) -> Result<()> {
        fs::create_dir_all(root)?;

        for entry in tree.iter() {
            let entry = entry?;
            let path = root.join(entry.filename().to_str_lossy().as_ref());

            match entry.kind() {
                tree::EntryKind::Blob | tree::EntryKind::BlobExecutable => {
                    let object = repo.find_object(entry.oid())?; // Note: .oid() on EntryRef
                    let blob = object.peel_to_kind(Kind::Blob)?;
                    fs::write(&path, blob.data.as_slice())?;
                }
                tree::EntryKind::Tree => {
                    let object = repo.find_object(entry.oid())?;
                    let subtree = object.into_tree();
                    self.extract_tree(repo, &subtree, &path)?;
                }
                _ => {} // Ignore Symlinks (Link) and Submodules (Commit)
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct IndicatifProgress {
    // Shared MultiProgress to ensure bars stack correctly
    mp: Arc<MultiProgress>,
    // The specific bar for this task
    bar: ProgressBar,
    // Keep track of name for messages
    name: Arc<Mutex<String>>,
}

impl IndicatifProgress {
    pub fn new() -> Self {
        let mp = Arc::new(MultiProgress::new());
        let bar = mp.add(ProgressBar::new(0));

        // Default style (spinner) until init() is called
        bar.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap());

        Self {
            mp,
            bar,
            name: Arc::new(Mutex::new(String::from("Initializing"))),
        }
    }

    // Helper to create child instances
    fn new_child(&self, name: String) -> Self {
        let bar = self.mp.add(ProgressBar::new(0));
        bar.set_message(name.clone());
        bar.set_style(ProgressStyle::with_template("  {spinner:.green} {msg}").unwrap());

        Self {
            mp: self.mp.clone(),
            bar,
            name: Arc::new(Mutex::new(name)),
        }
    }
}

// 1. Implement Count (Atomic updates to the bar position)
impl Count for IndicatifProgress {
    fn set(&self, step: usize) {
        self.bar.set_position(step as u64);
    }

    fn step(&self) -> usize {
        self.bar.position() as usize
    }

    fn inc_by(&self, step: usize) {
        self.bar.inc(step as u64);
    }

    fn counter(&self) -> Arc<AtomicUsize> {
        Arc::new(AtomicUsize::new(self.step()))
    }
}

// 2. Implement Progress (The trait you provided)
impl Progress for IndicatifProgress {
    fn init(&mut self, max: Option<usize>, unit: Option<Unit>) {
        // Set the max length (or spinner if None)
        if let Some(max) = max {
            self.bar.set_length(max as u64);
        }

        let mut is_bytes = false;

        if let Some(u) = &unit {
            let mut buf = String::new();
            if u.as_display_value().display_unit(&mut buf, 1).is_ok() {
                if buf == "B" || buf.to_lowercase().contains("byte") {
                    is_bytes = true;
                }
            }
        }

        // Fallback: If unit is missing, check the name
        let name_implies_bytes = self.name.lock().unwrap().contains("Receiving objects");

        let template = if is_bytes || name_implies_bytes {
            // Download style: [===>   ] 5MB/10MB (Speed)
            "{spinner:.green} {msg} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
        } else {
            // Count style: [===>   ] 50/100 (Speed)
            "{spinner:.green} {msg} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({per_sec}, {eta})"
        };

        if let Ok(style) = ProgressStyle::with_template(template) {
            self.bar.set_style(style.progress_chars("#>-"));
        }
    }

    fn set_name(&mut self, name: String) {
        *self.name.lock().unwrap() = name.clone();
        self.bar.set_message(name);
    }

    fn name(&self) -> Option<String> {
        Some(self.name.lock().unwrap().clone())
    }

    fn id(&self) -> Id {
        Id::default() // Not strictly used for display
    }

    fn message(&self, level: gix::progress::MessageLevel, message: String) {
        // Print message above the bar so it doesn't break the layout
        let level_prefix = match level {
            gix::progress::MessageLevel::Failure => "❌",
            gix::progress::MessageLevel::Success => "✅",
            _ => "ℹ️",
        };
        self.bar.println(format!("{} {}", level_prefix, message));
    }
}

// 3. Implement NestedProgress (Required for 'fetch' to spawn children)
impl NestedProgress for IndicatifProgress {
    type SubProgress = IndicatifProgress;

    fn add_child(&mut self, name: impl Into<String>) -> Self::SubProgress {
        self.new_child(name.into())
    }

    fn add_child_with_id(&mut self, name: impl Into<String>, _id: Id) -> Self::SubProgress {
        self.new_child(name.into())
    }
}
