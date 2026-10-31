//! Asset packs: swappable themes for the game's textures, sounds and models.
//!
//! A pack is a folder `assets/packs/<id>/` laid out like `assets/` itself
//! (`textures/`, `audio/`, `models/`) with a `pack.json` (name, author,
//! licence, credits, optional `extends`). Code keeps loading plain paths such
//! as `textures/blocks/stone.png`; the reader looks in the active pack, then
//! the packs it extends, then the free pack, then `assets/` (shaders, configs,
//! icon). A pack therefore only needs the files it changes.
//!
//! Packs switch while the game runs (Settings > Theme or a [`SwitchPack`]
//! message): the lookup order changes, every loaded texture, sound and model
//! reloads, and images built from them ([`DerivedImages`]) are rebuilt. The
//! pick is saved in `asset-pack.json`; `MINEHEADS_PACK=<id>` overrides it for
//! one run. Pack folders are found at startup.

use crate::components::settings_menu::{SettingsPage, SettingsPageRoot, ToggleButtonText};
use crate::resources::settings::{persistence_enabled, read_settings_file, write_settings_file};
use crate::states::GameState;
use bevy::asset::io::file::FileAssetReader;
use bevy::asset::io::{
    AssetReader, AssetReaderError, AssetSourceBuilder, AssetSourceId, PathStream, Reader,
};
use bevy::picking::hover::PickingInteraction;
use bevy::prelude::*;
use bevy::world_serialization::WorldAsset;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// The pack that ships with the public code; every other pack falls back to it.
pub const FREE_PACK: &str = "free";
const PACK_CHOICE_FILE: &str = "asset-pack.json";
const PACKS_DIR: &str = "assets/packs";
/// Asset folders that packs provide; anything else (shaders, configs) is shared.
const PACK_FOLDERS: [&str; 4] = ["textures", "audio", "models", "icon"];

#[derive(Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct PackManifest {
    #[serde(skip)]
    pub id: String,
    pub name: String,
    pub author: String,
    pub license: String,
    pub extends: Option<String>,
    pub credits: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct PackChoice {
    pack: Option<String>,
}

/// Installed packs and the active one. `order` is shared with the asset reader:
/// indices into `installed`, then `installed.len()` for `assets/` itself.
#[derive(Resource, Clone, Debug)]
pub struct AssetPacks {
    pub installed: Vec<PackManifest>,
    pub active: String,
    /// Lookup order: active pack, packs it extends, then the free pack.
    pub chain: Vec<String>,
    order: Arc<RwLock<Vec<usize>>>,
}

impl AssetPacks {
    fn name(&self, id: &str) -> String {
        self.installed
            .iter()
            .find(|p| p.id == id)
            .map_or_else(|| id.to_string(), |p| p.name.clone())
    }

    fn layer_order(&self) -> Vec<usize> {
        let mut order: Vec<usize> = self
            .chain
            .iter()
            .filter_map(|id| self.installed.iter().position(|p| &p.id == id))
            .collect();
        order.push(self.installed.len());
        order
    }
}

/// Switch to the pack with this id while the game runs.
#[derive(Message, Clone, Debug)]
pub struct SwitchPack(pub String);

/// Reads every `assets/packs/*/pack.json`, sorted by id.
fn installed_packs(packs_dir: &Path) -> Vec<PackManifest> {
    let mut packs: Vec<PackManifest> = std::fs::read_dir(packs_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let id = entry.file_name().to_str()?.to_string();
            let json = std::fs::read_to_string(entry.path().join("pack.json")).ok()?;
            let mut manifest: PackManifest = serde_json::from_str(&json)
                .inspect_err(|e| warn!("Ignoring asset pack {id}: bad pack.json ({e})"))
                .ok()?;
            if manifest.name.is_empty() {
                manifest.name = id.clone();
            }
            manifest.id = id;
            Some(manifest)
        })
        .collect();
    packs.sort_by(|a, b| a.id.cmp(&b.id));
    packs
}

/// Without a saved pick, a theme someone installed wins over the free pack.
fn default_pack(installed: &[PackManifest]) -> String {
    installed
        .iter()
        .find(|p| p.id != FREE_PACK)
        .map_or_else(|| FREE_PACK.to_string(), |p| p.id.clone())
}

/// Picks the pack for this run and its lookup chain.
fn resolve(requested: Option<String>, installed: &[PackManifest]) -> (String, Vec<String>) {
    let is_installed = |id: &str| installed.iter().any(|p| p.id == id);
    let active = match requested {
        Some(id) if is_installed(&id) => id,
        Some(id) => {
            warn!("Asset pack '{id}' is not installed; using the default pack");
            default_pack(installed)
        }
        None => default_pack(installed),
    };
    let mut chain = vec![active.clone()];
    let mut next = installed
        .iter()
        .find(|p| p.id == active)
        .and_then(|p| p.extends.clone());
    while let Some(id) = next.take() {
        if chain.contains(&id) || !is_installed(&id) {
            break;
        }
        next = installed
            .iter()
            .find(|p| p.id == id)
            .and_then(|p| p.extends.clone());
        chain.push(id);
    }
    if !chain.iter().any(|id| id == FREE_PACK) {
        chain.push(FREE_PACK.to_string());
    }
    (active, chain)
}

fn saved_choice() -> Option<String> {
    if !persistence_enabled() {
        return None;
    }
    read_settings_file(PACK_CHOICE_FILE)
        .and_then(|s| serde_json::from_str::<PackChoice>(&s).ok())
        .and_then(|c| c.pack)
}

fn save_choice(id: &str) {
    if persistence_enabled()
        && let Ok(json) = serde_json::to_string_pretty(&PackChoice {
            pack: Some(id.to_string()),
        })
    {
        write_settings_file(PACK_CHOICE_FILE, &json);
    }
}

/// Asset reader over every installed pack plus `assets/`, tried in the current
/// order. The order is swapped at runtime; the readers themselves stay put.
struct PackReader {
    layers: Vec<FileAssetReader>,
    order: Arc<RwLock<Vec<usize>>>,
}

impl PackReader {
    fn order(&self) -> Vec<usize> {
        self.order.read().map(|o| o.clone()).unwrap_or_default()
    }
}

impl AssetReader for PackReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<Box<dyn Reader + 'a>, AssetReaderError> {
        for i in self.order() {
            match self.layers[i].read(path).await {
                Ok(reader) => return Ok(Box::new(reader)),
                Err(AssetReaderError::NotFound(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(AssetReaderError::NotFound(path.to_path_buf()))
    }

    async fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<dyn Reader + 'a>, AssetReaderError> {
        for i in self.order() {
            match self.layers[i].read_meta(path).await {
                Ok(reader) => return Ok(Box::new(reader)),
                Err(AssetReaderError::NotFound(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(AssetReaderError::NotFound(path.to_path_buf()))
    }

    /// Folders are not merged: the first layer that has the folder lists it.
    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        for i in self.order() {
            if self.layers[i].is_directory(path).await.unwrap_or(false) {
                return self.layers[i].read_directory(path).await;
            }
        }
        Err(AssetReaderError::NotFound(path.to_path_buf()))
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        for i in self.order() {
            if self.layers[i].is_directory(path).await.unwrap_or(false) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Registers the pack reader as the default asset source. Call before
/// `DefaultPlugins` (Bevy's `AssetPlugin` keeps an already registered source).
pub fn install(app: &mut App) {
    let packs_dir = FileAssetReader::get_base_path().join(PACKS_DIR);
    let installed = installed_packs(&packs_dir);
    let requested = std::env::var("MINEHEADS_PACK")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(saved_choice);
    let (active, chain) = resolve(requested, &installed);
    let mut packs = AssetPacks {
        installed,
        active,
        chain,
        order: Arc::default(),
    };
    packs.order = Arc::new(RwLock::new(packs.layer_order()));
    let mut roots: Vec<PathBuf> = packs
        .installed
        .iter()
        .map(|p| Path::new(PACKS_DIR).join(&p.id))
        .collect();
    roots.push(PathBuf::from("assets"));
    let order = packs.order.clone();
    app.register_asset_source(
        AssetSourceId::Default,
        AssetSourceBuilder::new(move || {
            Box::new(PackReader {
                layers: roots.iter().map(FileAssetReader::new).collect(),
                order: order.clone(),
            })
        }),
    );
    app.insert_resource(packs);
}

/// Builds a derived image from its loaded source images.
type AtlasBuilder = Box<dyn Fn(&[&Image]) -> Image + Send + Sync>;

/// An image built from loaded images (atlases); rebuilt when a source reloads.
struct Derived {
    target: Handle<Image>,
    sources: Vec<Handle<Image>>,
    build: AtlasBuilder,
}

/// Images made from pack textures at runtime. Register them here so a pack
/// switch refreshes them in place (materials keep the same handle).
#[derive(Resource, Default)]
pub struct DerivedImages(Vec<Derived>);

impl DerivedImages {
    pub fn add(
        &mut self,
        target: Handle<Image>,
        sources: Vec<Handle<Image>>,
        build: impl Fn(&[&Image]) -> Image + Send + Sync + 'static,
    ) {
        self.0.push(Derived {
            target,
            sources,
            build: Box::new(build),
        });
    }
}

#[derive(Component)]
pub struct PackButton(pub String);

/// Holds the pack buttons and credits; refilled when the active pack changes.
#[derive(Component)]
struct ThemeList;

pub struct AssetPacksPlugin;

impl Plugin for AssetPacksPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SwitchPack>()
            .init_resource::<DerivedImages>()
            .add_systems(Startup, log_active_pack)
            .add_systems(Update, (apply_pack_switch, rebuild_derived_images).chain())
            .add_systems(
                Update,
                (fill_theme_page, refill_theme_list, press_pack_button)
                    .chain()
                    .run_if(in_state(GameState::Settings)),
            );
    }
}

/// Logged at startup: `install` runs before the log is set up.
fn log_active_pack(packs: Option<Res<AssetPacks>>) {
    if let Some(packs) = packs {
        info!(
            "Asset pack: {} (lookup order: {})",
            packs.active,
            packs.chain.join(" > ")
        );
    }
}

/// Paths of every loaded texture, sound and model that a pack provides.
fn pack_asset_paths(
    server: &AssetServer,
    images: &Assets<Image>,
    sounds: &Assets<AudioSource>,
    worlds: &Assets<WorldAsset>,
) -> Vec<String> {
    let ids = images
        .ids()
        .map(|id| id.untyped())
        .chain(sounds.ids().map(|id| id.untyped()))
        .chain(worlds.ids().map(|id| id.untyped()));
    let mut paths: Vec<String> = ids
        .filter_map(|id| server.get_path(id))
        .map(|p| p.path().to_string_lossy().replace('\\', "/"))
        .filter(|p| {
            p.split('/')
                .next()
                .is_some_and(|top| PACK_FOLDERS.contains(&top))
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    paths.sort();
    paths
}

fn apply_pack_switch(
    mut requests: MessageReader<SwitchPack>,
    packs: Option<ResMut<AssetPacks>>,
    server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    sounds: Res<Assets<AudioSource>>,
    worlds: Res<Assets<WorldAsset>>,
) {
    let Some(request) = requests.read().last().cloned() else {
        return;
    };
    let Some(mut packs) = packs else { return };
    let (active, chain) = resolve(Some(request.0), &packs.installed);
    if active == packs.active {
        return;
    }
    packs.active = active;
    packs.chain = chain;
    let order = packs.layer_order();
    if let Ok(mut shared) = packs.order.write() {
        *shared = order;
    }
    save_choice(&packs.active);
    let paths = pack_asset_paths(&server, &images, &sounds, &worlds);
    info!(
        "Theme pack switched to {} ({}); reloading {} assets",
        packs.active,
        packs.chain.join(" > "),
        paths.len()
    );
    for path in paths {
        server.reload(path);
    }
}

/// Rebuilds registered atlases when one of their source images changed.
fn rebuild_derived_images(
    mut events: MessageReader<AssetEvent<Image>>,
    derived: Res<DerivedImages>,
    mut images: ResMut<Assets<Image>>,
) {
    let changed: HashSet<AssetId<Image>> = events
        .read()
        .filter_map(|e| match e {
            AssetEvent::Modified { id } | AssetEvent::LoadedWithDependencies { id } => Some(*id),
            _ => None,
        })
        .collect();
    if changed.is_empty() {
        return;
    }
    for d in &derived.0 {
        if !d.sources.iter().any(|s| changed.contains(&s.id())) {
            continue;
        }
        let rebuilt = {
            let sources: Option<Vec<&Image>> = d.sources.iter().map(|s| images.get(s)).collect();
            sources.map(|s| (d.build)(&s))
        };
        if let Some(image) = rebuilt {
            let _ = images.insert(&d.target, image);
        }
    }
}

fn small_text(text: impl Into<String>) -> impl Bundle {
    (
        bevy::picking::Pickable::IGNORE,
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(13.),
            ..default()
        },
        TextColor(Color::srgb(0.7, 0.75, 0.78)),
    )
}

fn pack_label(packs: &AssetPacks, id: &str) -> String {
    if id == packs.active {
        format!("> {} (playing)", packs.name(id))
    } else {
        packs.name(id)
    }
}

/// Adds the intro text and an empty list to the Theme settings page.
fn fill_theme_page(
    mut commands: Commands,
    pages: Query<(Entity, &SettingsPageRoot), Added<SettingsPageRoot>>,
) {
    for (page, root) in &pages {
        if root.0 != SettingsPage::Theme {
            continue;
        }
        commands.entity(page).with_children(|p| {
            p.spawn(small_text(
                "Pick the textures, sounds and models. Switches right away and is saved.",
            ));
            p.spawn((
                ThemeList,
                Node {
                    width: Val::Percent(100.),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.),
                    ..default()
                },
            ));
        });
    }
}

/// One button per installed pack, then the credits of the active pack.
fn refill_theme_list(
    mut commands: Commands,
    lists: Query<Entity, With<ThemeList>>,
    added: Query<(), Added<ThemeList>>,
    packs: Option<Res<AssetPacks>>,
) {
    let Some(packs) = packs else { return };
    if added.is_empty() && !packs.is_changed() {
        return;
    }
    for list in &lists {
        commands.entity(list).despawn_children().with_children(|p| {
            for pack in &packs.installed {
                crate::systems::settings_menu_system::menu_button(
                    p,
                    &pack_label(&packs, &pack.id),
                    PackButton(pack.id.clone()),
                );
            }
            if let Some(active) = packs.installed.iter().find(|m| m.id == packs.active) {
                let by = if active.author.is_empty() {
                    String::new()
                } else {
                    format!(" by {}", active.author)
                };
                p.spawn(small_text(format!("Now playing: {}{by}", active.name)));
                if !active.license.is_empty() {
                    p.spawn(small_text(format!("Licence: {}", active.license)));
                }
                for credit in &active.credits {
                    p.spawn(small_text(format!("- {credit}")));
                }
            }
        });
    }
}

type PressedPackButton = (Changed<PickingInteraction>, With<PackButton>);

fn press_pack_button(
    buttons: Query<(&PickingInteraction, &PackButton, &Children), PressedPackButton>,
    mut texts: Query<&mut Text, With<ToggleButtonText>>,
    mut switch: MessageWriter<SwitchPack>,
) {
    for (interaction, button, children) in &buttons {
        if *interaction != PickingInteraction::Pressed {
            continue;
        }
        switch.write(SwitchPack(button.0.clone()));
        for child in children {
            if let Ok(mut text) = texts.get_mut(*child) {
                text.0 = "Switching...".into();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack(id: &str, extends: Option<&str>) -> PackManifest {
        PackManifest {
            id: id.into(),
            name: id.into(),
            extends: extends.map(Into::into),
            ..default()
        }
    }

    #[test]
    fn chain_ends_in_free_pack_and_survives_bad_input() {
        let installed = vec![
            pack("free", None),
            pack("minecraft", None),
            pack("winter", Some("minecraft")),
        ];
        // No pick: an installed theme wins over the free pack.
        assert_eq!(resolve(None, &installed).0, "minecraft");
        // Extends chain, then the free pack.
        assert_eq!(
            resolve(Some("winter".into()), &installed).1,
            ["winter", "minecraft", "free"]
        );
        // Unknown pick falls back to the default.
        assert_eq!(resolve(Some("gone".into()), &installed).0, "minecraft");
        // Only the free pack installed.
        assert_eq!(resolve(None, &installed[..1]).1, ["free"]);
        // A cycle does not loop forever.
        let cyclic = vec![
            pack("a", Some("b")),
            pack("b", Some("a")),
            pack("free", None),
        ];
        assert_eq!(resolve(Some("a".into()), &cyclic).1, ["a", "b", "free"]);
    }

    #[test]
    fn layer_order_follows_chain_then_shared_assets() {
        let installed = vec![
            pack("free", None),
            pack("minecraft", None),
            pack("winter", Some("minecraft")),
        ];
        let (active, chain) = resolve(Some("winter".into()), &installed);
        let packs = AssetPacks {
            installed,
            active,
            chain,
            order: Arc::default(),
        };
        // winter (2) > minecraft (1) > free (0) > assets/ (3)
        assert_eq!(packs.layer_order(), [2, 1, 0, 3]);
    }
}
