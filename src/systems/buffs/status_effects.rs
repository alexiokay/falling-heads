//! Makes active effects visible: a slime overlay on heads coated by a slimeball
//! (like the fire overlay for Burning) and an "Effects" line on the HUD that
//! lists every effect on the hero with the time left.
use crate::components::buff::{Buff, BuffBehavior, GrowthPower};
use crate::components::frozen::FrozenStacks;
use crate::components::hero::Hero;
use crate::components::power_effect::{CobwebSlow, SpeedModifier};
use crate::systems::slime_physics::SlimeCoat;
use bevy::prelude::*;

/// Last seconds of the slime coat, during which the overlay blinks.
const SLIME_WARNING: f32 = 1.5;

pub struct StatusEffectsPlugin;

impl Plugin for StatusEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            slime_overlays
                .after(crate::systems::biome_teleport::finish_biome_teleport)
                .before(TransformSystems::Propagate),
        )
        .add_systems(Update, hero_effects_hud);
    }
}

#[derive(Component)]
struct SlimeOverlay {
    owner: Entity,
}

/// HUD line listing the hero's active effects.
#[derive(Component)]
pub struct HeroEffectsText;

/// Adds, moves, blinks and removes the green slime layer on coated heads.
#[allow(clippy::too_many_arguments)] // Bevy system parameters
fn slime_overlays(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    asset_server: Res<AssetServer>,
    coated: Query<(Entity, &SlimeCoat, &Transform, Option<&Mesh3d>), Without<SlimeOverlay>>,
    mut overlays: Query<
        (
            Entity,
            &SlimeOverlay,
            &mut Transform,
            &MeshMaterial3d<StandardMaterial>,
        ),
        Without<SlimeCoat>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut quad: Local<Option<Handle<Mesh>>>,
) {
    let now = time.elapsed_secs();
    let mut covered = std::collections::HashSet::new();
    for (entity, overlay, mut transform, material) in &mut overlays {
        let Ok((_, coat, owner, face)) = coated.get(overlay.owner) else {
            commands.entity(entity).despawn();
            continue;
        };
        let left = coat.until - now;
        if left <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        covered.insert(overlay.owner);
        let bounds = crate::systems::head_geometry::bounds(&meshes, face);
        *transform = crate::systems::head_geometry::overlay_transform(bounds, owner);
        transform.translation.z += crate::systems::actor_depth::ICE_OVERLAY + 1.0;
        // Blink while the coat is about to run out.
        let alpha = if left < SLIME_WARNING && (now * 8.0) as i32 % 2 == 0 {
            0.15
        } else {
            0.55
        };
        if let Some(m) = materials.get(&material.0)
            && m.base_color.alpha() != alpha
            && let Some(mut m) = materials.get_mut(&material.0)
        {
            m.base_color.set_alpha(alpha);
        }
    }
    for (entity, coat, owner, face) in &coated {
        if covered.contains(&entity) || coat.until <= now {
            continue;
        }
        let mesh = quad
            .get_or_insert_with(|| meshes.add(Rectangle::new(64.0, 64.0)))
            .clone();
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.6, 1.0, 0.55, 0.55),
            base_color_texture: Some(asset_server.load("textures/biome_content/slime_block.png")),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        let bounds = crate::systems::head_geometry::bounds(&meshes, face);
        let mut transform = crate::systems::head_geometry::overlay_transform(bounds, owner);
        transform.translation.z += crate::systems::actor_depth::ICE_OVERLAY + 1.0;
        commands.spawn((
            SlimeOverlay { owner: entity },
            Mesh3d(mesh),
            MeshMaterial3d(material),
            transform,
            bevy::light::NotShadowCaster,
        ));
    }
}

/// Text for the hero's effects, e.g. "Slime 6s  Burning 3s  Fast x2 4s".
pub(crate) fn effects_line(
    buff: Option<&Buff>,
    speed: Option<&SpeedModifier>,
    frozen: Option<&FrozenStacks>,
    coat_left: Option<f32>,
    growth: Option<&GrowthPower>,
    webbed: bool,
) -> String {
    let mut parts = Vec::new();
    if let Some(left) = coat_left.filter(|l| *l > 0.0) {
        parts.push(format!("Slime {}s", left.ceil()));
    }
    if let Some(buff) = buff {
        let stacks = if buff.stacks > 1 {
            format!(" x{}", buff.stacks)
        } else {
            String::new()
        };
        parts.push(format!(
            "{}{stacks} {}s",
            buff.buff_type.name(),
            buff.duration.remaining_secs().ceil()
        ));
    }
    if let Some(speed) = speed {
        let name = if speed.multiplier >= 1.0 {
            "Fast"
        } else {
            "Slow"
        };
        parts.push(format!(
            "{name} x{} {}s",
            speed.multiplier,
            speed.duration.remaining_secs().ceil()
        ));
    }
    if let Some(frozen) = frozen {
        parts.push(format!("Frozen {}/6", frozen.stack_count()));
    }
    if let Some(growth) = growth {
        parts.push(format!("Grow {}s", growth.duration.remaining_secs().ceil()));
    }
    if webbed {
        parts.push("Webbed".to_string());
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("Effects: {}", parts.join("  "))
    }
}

#[allow(clippy::type_complexity)]
fn hero_effects_hud(
    time: Res<Time<Virtual>>,
    hero: Query<
        (
            Option<&Buff>,
            Option<&SpeedModifier>,
            Option<&FrozenStacks>,
            Option<&SlimeCoat>,
            Option<&GrowthPower>,
            Has<CobwebSlow>,
        ),
        With<Hero>,
    >,
    mut texts: Query<&mut Text, With<HeroEffectsText>>,
) {
    let Ok((buff, speed, frozen, coat, growth, webbed)) = hero.single() else {
        return;
    };
    let now = time.elapsed_secs();
    let line = effects_line(
        buff,
        speed,
        frozen,
        coat.map(|c| c.until - now),
        growth,
        webbed,
    );
    for mut text in &mut texts {
        if text.0 != line {
            text.0 = line.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::buff::BuffType;

    #[test]
    fn effects_line_lists_every_active_effect_with_time_left() {
        assert_eq!(effects_line(None, None, None, None, None, false), "");
        let buff = Buff::new(BuffType::Burning, 3.0);
        let speed = SpeedModifier {
            multiplier: 2.0,
            duration: Timer::from_seconds(5.0, TimerMode::Once),
        };
        let line = effects_line(Some(&buff), Some(&speed), None, Some(5.2), None, true);
        assert_eq!(line, "Effects: Slime 6s  Burning 3s  Fast x2 5s  Webbed");
        // An expired coat is not shown.
        assert_eq!(effects_line(None, None, None, Some(-0.1), None, false), "");
    }
}
