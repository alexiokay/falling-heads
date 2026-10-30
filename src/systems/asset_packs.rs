//! Asset packs: swappable themes for the game's textures, sounds and models.
//!
//! A pack is a folder `assets/packs/<id>/` laid out like `assets/` itself
//! (`textures/`, `audio/`, `models/`) with a `pack.json` (name, author,
//! licence, credits, optional `extends`). Code keeps loading plain paths such
//! as `textures/blocks/stone.png`; the reader looks in the active pack, then
//! the packs it extends, then the free pack, then `assets/` (shaders, configs,
//! icon). A pack therefore only needs the files it changes.
//!
//! The pick is saved in `asset-pack.json` and applies on the next start.
//! `MINEHEADS_PACK=<id>` overrides it for one run (used to test the free pack).

use crate::components::settings_menu::{SettingsPage, SettingsPageRoot, ToggleButtonText};
use crate::resources::settings::{persistence_enabled, read_settings_file, write_settings_file};
use crate::states::GameState;
use bevy::asset::io::file::FileAssetReader;
use bevy::asset::io::{
    AssetReader, AssetReaderError, AssetSourceBuilder, AssetSourceId, PathStream, Reader,
};
use bevy::picking::hover::PickingInteraction;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The pack that ships with the public code; every other pack falls back to it.
pub const FREE_PACK: &str = "free";
const PACK_CHOICE_FILE: &str = "asset-pack.json";
const PACKS_DIR: &str = "assets/packs";

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

/// Installed packs, the one this run uses, and the one picked for next start.
#[derive(Resource, Clone, Debug)]
pub struct AssetPacks {
    pub installed: Vec<PackManifest>,
    pub active: String,
    /// Lookup order: active pack, packs it extends, then the free pack.
    pub chain: Vec<String>,
    pub next: String,
}

impl AssetPacks {
    fn name(&self, id: &str) -> String {
        self.installed
            .iter()
            .find(|p| p.id == id)
            .map_or_else(|| id.to_string(), |p| p.name.clone())
    }
}

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

/// Asset reader that tries each pack in the chain, then `assets/` itself.
struct PackReader {
    layers: Vec<FileAssetReader>,
}

impl AssetReader for PackReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<Box<dyn Reader + 'a>, AssetReaderError> {
        for layer in &self.layers {
            match layer.read(path).await {
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
        for layer in &self.layers {
            match layer.read_meta(path).await {
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
        for layer in &self.layers {
            if layer.is_directory(path).await.unwrap_or(false) {
                return layer.read_directory(path).await;
            }
        }
        Err(AssetReaderError::NotFound(path.to_path_buf()))
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        for layer in &self.layers {
            if layer.is_directory(path).await.unwrap_or(false) {
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
    let mut roots: Vec<PathBuf> = chain
        .iter()
        .map(|id| Path::new(PACKS_DIR).join(id))
        .collect();
    roots.push(PathBuf::from("assets"));
    app.register_asset_source(
        AssetSourceId::Default,
        AssetSourceBuilder::new(move || {
            Box::new(PackReader {
                layers: roots.iter().map(FileAssetReader::new).collect(),
            })
        }),
    );
    app.insert_resource(AssetPacks {
        installed,
        next: active.clone(),
        active,
        chain,
    });
}

#[derive(Component)]
pub struct PackCycleButton;

pub struct AssetPacksPlugin;

impl Plugin for AssetPacksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, log_active_pack).add_systems(
            Update,
            (fill_theme_page, cycle_pack)
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

fn pack_label(packs: &AssetPacks) -> String {
    let mut label = format!("Theme: {}", packs.name(&packs.next));
    if packs.next != packs.active {
        label.push_str(" (restart to apply)");
    }
    label
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

/// Fills the Theme settings page: a cycle button and the credits of the active pack.
fn fill_theme_page(
    mut commands: Commands,
    pages: Query<(Entity, &SettingsPageRoot), Added<SettingsPageRoot>>,
    packs: Option<Res<AssetPacks>>,
) {
    let Some(packs) = packs else { return };
    for (page, root) in &pages {
        if root.0 != SettingsPage::Theme {
            continue;
        }
        commands.entity(page).with_children(|p| {
            p.spawn(small_text(
                "Textures, sounds and models come from the theme pack. Saved for next start.",
            ));
            crate::systems::settings_menu_system::menu_button(
                p,
                &pack_label(&packs),
                PackCycleButton,
            );
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

type PressedPackButton = (Changed<PickingInteraction>, With<PackCycleButton>);

fn cycle_pack(
    packs: Option<ResMut<AssetPacks>>,
    buttons: Query<(&PickingInteraction, &Children), PressedPackButton>,
    mut texts: Query<&mut Text, With<ToggleButtonText>>,
) {
    let Some(mut packs) = packs else { return };
    for (interaction, children) in &buttons {
        if *interaction != PickingInteraction::Pressed || packs.installed.is_empty() {
            continue;
        }
        let at = packs
            .installed
            .iter()
            .position(|p| p.id == packs.next)
            .unwrap_or(0);
        packs.next = packs.installed[(at + 1) % packs.installed.len()].id.clone();
        info!("Theme pack for next start: {}", packs.next);
        if persistence_enabled()
            && let Ok(json) = serde_json::to_string_pretty(&PackChoice {
                pack: Some(packs.next.clone()),
            })
        {
            write_settings_file(PACK_CHOICE_FILE, &json);
        }
        let label = pack_label(&packs);
        for child in children {
            if let Ok(mut text) = texts.get_mut(*child) {
                text.0 = label.clone();
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
}
