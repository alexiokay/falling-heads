//! Character selection: the hero's face and play style, picked under
//! Settings > Character. Applies right away and is saved for next time. The
//! faces come from the active theme pack, so they follow a pack switch.
use crate::components::hero::Hero;
use crate::components::play_style::PlayStyle;
use crate::components::settings_menu::{SettingsPage, SettingsPageRoot};
use crate::resources::settings::{persistence_enabled, read_settings_file, write_settings_file};
use crate::states::GameState;
use bevy::picking::hover::PickingInteraction;
use bevy::prelude::*;
use bevy::ui_widgets::Button;
use serde::{Deserialize, Serialize};

const CHARACTER_FILE: &str = "character.json";

/// A selectable hero: id saved in settings, name shown, face texture path.
pub struct Character {
    pub id: &'static str,
    pub name: &'static str,
    pub face: &'static str,
}

pub const CHARACTERS: [Character; 6] = [
    Character {
        id: "classic",
        name: "Classic",
        face: "textures/entities/hero.jpg",
    },
    Character {
        id: "miner",
        name: "Miner",
        face: "textures/entities/player/steve.jpg",
    },
    Character {
        id: "ember",
        name: "Ember",
        face: "textures/entities/player/skinmc-avatar (3).png",
    },
    Character {
        id: "sunny",
        name: "Sunny",
        face: "textures/entities/player/skinmc-avatar (4).png",
    },
    Character {
        id: "villager",
        name: "Villager",
        face: "textures/entities/villager.png",
    },
    Character {
        id: "penguin",
        name: "Penguin",
        face: "textures/entities/enemy/penguin.png",
    },
];

fn style_blurb(style: PlayStyle) -> &'static str {
    match style {
        PlayStyle::Bouncy => "Bouncy: full bounces off everything.",
        PlayStyle::Heavy => "Heavy: falls fast, thuds and digs down hard.",
        PlayStyle::Floaty => "Floaty: low gravity, long drifting hops.",
        PlayStyle::Sticky => "Sticky: grabs walls, then wall-jumps away.",
        PlayStyle::Splitter => "Splitter: a hard landing splits it in two for a few seconds.",
    }
}

#[derive(Serialize, Deserialize, Default)]
struct SavedChoice {
    character: Option<String>,
    style: Option<String>,
}

/// The hero's chosen face and play style.
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct ChosenCharacter {
    pub character: usize,
    pub style: PlayStyle,
}

impl ChosenCharacter {
    fn load() -> Self {
        let saved: SavedChoice = if persistence_enabled() {
            read_settings_file(CHARACTER_FILE)
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            SavedChoice::default()
        };
        Self {
            character: saved
                .character
                .and_then(|id| CHARACTERS.iter().position(|c| c.id == id))
                .unwrap_or(0),
            style: saved
                .style
                .and_then(|s| PlayStyle::parse(&s))
                .unwrap_or_default(),
        }
    }

    fn save(&self) {
        if !persistence_enabled() {
            return;
        }
        let saved = SavedChoice {
            character: Some(CHARACTERS[self.character].id.to_string()),
            style: Some(self.style.name().to_string()),
        };
        if let Ok(json) = serde_json::to_string_pretty(&saved) {
            write_settings_file(CHARACTER_FILE, &json);
        }
    }
}

#[derive(Component)]
pub struct CharacterButtonId(pub usize);

#[derive(Component)]
pub struct StyleButtonId(pub PlayStyle);

/// Holds the character grid and style buttons; refilled when the choice changes.
#[derive(Component)]
struct CharacterList;

pub struct CharacterSelectPlugin;

impl Plugin for CharacterSelectPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChosenCharacter::load())
            .add_systems(Update, (apply_choice, follow_hero_style).chain())
            .add_systems(
                Update,
                (
                    fill_character_page,
                    refill_character_list,
                    press_buttons,
                    color_buttons,
                )
                    .chain()
                    .run_if(in_state(GameState::Settings)),
            );
    }
}

/// The hero's face material and current play style.
type HeroLook<'a> = (
    Entity,
    &'a MeshMaterial3d<StandardMaterial>,
    Option<&'a PlayStyle>,
);

/// Puts the chosen face and style on the hero when it appears or the choice changes.
fn apply_choice(
    mut commands: Commands,
    chosen: Res<ChosenCharacter>,
    heroes: Query<HeroLook, With<Hero>>,
    new_heroes: Query<(), Added<Hero>>,
    server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !chosen.is_changed() && new_heroes.is_empty() {
        return;
    }
    for (hero, material, style) in &heroes {
        if let Some(mut m) = materials.get_mut(&material.0) {
            m.base_color_texture = Some(server.load(CHARACTERS[chosen.character].face));
        }
        if style.copied() != Some(chosen.style) {
            commands.entity(hero).insert(chosen.style);
        }
    }
}

/// The C hotkey and the dev panel also change the hero's style: remember it.
fn follow_hero_style(
    mut chosen: ResMut<ChosenCharacter>,
    heroes: Query<&PlayStyle, (With<Hero>, Changed<PlayStyle>)>,
) {
    for style in &heroes {
        if *style != chosen.style {
            chosen.style = *style;
            chosen.save();
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

fn fill_character_page(
    mut commands: Commands,
    pages: Query<(Entity, &SettingsPageRoot), Added<SettingsPageRoot>>,
) {
    for (page, root) in &pages {
        if root.0 != SettingsPage::Character {
            continue;
        }
        commands.entity(page).with_children(|p| {
            p.spawn(small_text(
                "Pick your hero's face and play style. Applies right away and is saved.",
            ));
            p.spawn((
                CharacterList,
                Node {
                    width: Val::Percent(100.),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.),
                    ..default()
                },
            ));
        });
    }
}

fn refill_character_list(
    mut commands: Commands,
    lists: Query<Entity, With<CharacterList>>,
    added: Query<(), Added<CharacterList>>,
    chosen: Res<ChosenCharacter>,
    server: Res<AssetServer>,
) {
    if added.is_empty() && !chosen.is_changed() {
        return;
    }
    for list in &lists {
        commands.entity(list).despawn_children().with_children(|p| {
            // Faces: a wrapping grid of picture buttons.
            p.spawn(Node {
                width: Val::Percent(100.),
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(8.),
                row_gap: Val::Px(8.),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|grid| {
                for (i, character) in CHARACTERS.iter().enumerate() {
                    grid.spawn((
                        CharacterButtonId(i),
                        Button,
                        PickingInteraction::default(),
                        Node {
                            width: Val::Px(84.),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(6.)),
                            row_gap: Val::Px(4.),
                            border: UiRect::all(Val::Px(2.)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.10, 0.16, 0.19)),
                        BorderColor::all(Color::NONE),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            bevy::picking::Pickable::IGNORE,
                            ImageNode::new(server.load(character.face)),
                            Node {
                                width: Val::Px(56.),
                                height: Val::Px(56.),
                                ..default()
                            },
                        ));
                        b.spawn((
                            bevy::picking::Pickable::IGNORE,
                            Text::new(character.name),
                            TextFont {
                                font_size: FontSize::Px(14.),
                                ..default()
                            },
                        ));
                    });
                }
            });
            p.spawn(small_text("Play style"));
            p.spawn(Node {
                width: Val::Percent(100.),
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(6.),
                row_gap: Val::Px(6.),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|row| {
                for style in PlayStyle::ALL {
                    row.spawn((
                        StyleButtonId(style),
                        Button,
                        PickingInteraction::default(),
                        Node {
                            padding: UiRect::axes(Val::Px(10.), Val::Px(6.)),
                            border: UiRect::all(Val::Px(2.)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.10, 0.16, 0.19)),
                        BorderColor::all(Color::NONE),
                    ))
                    .with_child((
                        bevy::picking::Pickable::IGNORE,
                        Text::new(style.name()),
                        TextFont {
                            font_size: FontSize::Px(14.),
                            ..default()
                        },
                    ));
                }
            });
            p.spawn(small_text(style_blurb(chosen.style)));
        });
    }
}

type Pressed<T> = (Changed<PickingInteraction>, With<T>);

fn press_buttons(
    mut chosen: ResMut<ChosenCharacter>,
    characters: Query<(&PickingInteraction, &CharacterButtonId), Pressed<CharacterButtonId>>,
    styles: Query<(&PickingInteraction, &StyleButtonId), Pressed<StyleButtonId>>,
) {
    let mut next = chosen.clone();
    for (interaction, button) in &characters {
        if *interaction == PickingInteraction::Pressed {
            next.character = button.0;
        }
    }
    for (interaction, button) in &styles {
        if *interaction == PickingInteraction::Pressed {
            next.style = button.0;
        }
    }
    if next != *chosen {
        info!(
            "Character: {} ({})",
            CHARACTERS[next.character].name,
            next.style.name()
        );
        next.save();
        *chosen = next;
    }
}

/// Selected buttons get a bright border; hover lightens the background.
#[allow(clippy::type_complexity)]
fn color_buttons(
    chosen: Res<ChosenCharacter>,
    mut buttons: Query<(
        &PickingInteraction,
        &mut BackgroundColor,
        &mut BorderColor,
        Option<&CharacterButtonId>,
        Option<&StyleButtonId>,
    )>,
) {
    for (interaction, mut bg, mut border, character, style) in &mut buttons {
        let selected = match (character, style) {
            (Some(c), _) => c.0 == chosen.character,
            (_, Some(s)) => s.0 == chosen.style,
            _ => continue,
        };
        bg.0 = match interaction {
            PickingInteraction::Pressed => Color::srgb(0.20, 0.42, 0.46),
            PickingInteraction::Hovered => Color::srgb(0.16, 0.28, 0.31),
            PickingInteraction::None if selected => Color::srgb(0.13, 0.32, 0.36),
            PickingInteraction::None => Color::srgb(0.10, 0.16, 0.19),
        };
        *border = BorderColor::all(if selected {
            Color::srgb(0.55, 0.95, 1.0)
        } else {
            Color::NONE
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn characters_have_unique_ids_and_every_style_a_blurb() {
        let mut ids: Vec<&str> = CHARACTERS.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), CHARACTERS.len());
        assert!(style_blurb(PlayStyle::Splitter).starts_with("Splitter"));
    }
}
