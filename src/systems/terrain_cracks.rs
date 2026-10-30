//! Damage belongs to terrain albedo in the RT G-buffer, not only a transparent card.
use crate::{
    components::{
        block::{Block, BlockType},
        damage_overlay::{BlockDamageOverlay, get_damage_stage},
    },
    resources::{asset_handles::DamageStageTextures, graphics_settings::GraphicsSettings},
};
use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};
#[derive(Asset, bevy::reflect::TypePath, AsBindGroup, Clone, Debug)]
pub struct CrackExtension {
    #[texture(100)]
    #[sampler(101)]
    pub texture: Handle<Image>,
    #[uniform(102)]
    pub parameters: Vec4,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn damage_survives_renderer_switch_and_releases_owned_materials() {
        let mut app = App::new();
        let mut settings = GraphicsSettings::default();
        settings.ray_tracing = true;
        app.insert_resource(settings)
            .insert_resource(super::super::native_gpu::NativeStatus {
                mode: 2,
                sr: true,
                rr: true,
                ray_query: true,
            })
            .insert_resource(DamageStageTextures {
                stages: std::array::from_fn(|_| Handle::default()),
            })
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Cracked>>()
            .init_resource::<Owned>()
            .add_systems(Update, update);
        let source = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let e = app
            .world_mut()
            .spawn((
                Block {
                    health: 2,
                    max_health: 5,
                },
                BlockType::Wood,
                Transform::default(),
                MeshMaterial3d(source.clone()),
            ))
            .id();
        app.update();
        verify(app.world(), &[e]);
        app.update();
        verify(app.world(), &[e]);
        app.world_mut().get_mut::<Block>(e).unwrap().health = 1;
        app.update();
        assert_eq!(app.world().resource::<Assets<Cracked>>().len(), 1);
        app.world_mut()
            .resource_mut::<GraphicsSettings>()
            .ray_tracing = false;
        app.update();
        app.update();
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(e)
                .unwrap()
                .0,
            source
        );
        assert!(app.world().get::<MeshMaterial3d<Cracked>>(e).is_none());
        assert_eq!(app.world().resource::<Assets<Cracked>>().len(), 0);
    }
}
impl MaterialExtension for CrackExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain_cracks.wesl".into()
    }
    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/terrain_cracks.wesl".into()
    }
}
pub(crate) type Cracked = ExtendedMaterial<StandardMaterial, CrackExtension>;
pub(crate) fn verify(world: &World, entities: &[Entity]) {
    for e in entities {
        eprintln!(
            "[RT CRACK CHECK] {e:?} health={:?} standard={} extended={}",
            world.get::<Block>(*e).map(|b| (b.health, b.max_health)),
            world.get::<MeshMaterial3d<StandardMaterial>>(*e).is_some(),
            world.get::<MeshMaterial3d<Cracked>>(*e).is_some()
        );
        assert!(
            world.get::<MeshMaterial3d<Cracked>>(*e).is_some(),
            "Damaged RT block must carry cracks in its G-buffer material"
        );
    }
}
#[derive(Component)]
struct Original {
    material: Handle<StandardMaterial>,
    stage: usize,
}
#[derive(Resource, Default)]
struct Owned(std::collections::HashMap<Entity, Handle<Cracked>>);
pub struct TerrainCracksPlugin;
impl Plugin for TerrainCracksPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<Cracked>::default())
            .init_resource::<Owned>()
            .add_systems(PostUpdate, update.before(TransformSystems::Propagate));
    }
}
fn update(
    mut commands: Commands,
    settings: Res<GraphicsSettings>,
    status: Res<super::native_gpu::NativeStatus>,
    textures: Res<DamageStageTextures>,
    base: Res<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<Cracked>>,
    mut owned: ResMut<Owned>,
    blocks: Query<(
        Entity,
        &Block,
        &BlockType,
        &Transform,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&Original>,
    )>,
    existing: Query<(), With<Original>>,
    mut overlays: Query<&mut Visibility, With<BlockDamageOverlay>>,
) {
    owned.0.retain(|e, h| {
        if existing.contains(*e) {
            true
        } else {
            materials.remove(h.id());
            false
        }
    });
    let rt = settings.ray_tracing && settings.lighting && status.mode == 2;
    if rt {
        for mut v in &mut overlays {
            *v = Visibility::Hidden;
        }
    }
    for (e, block, kind, t, standard, original) in &blocks {
        let stage = get_damage_stage(block.health, block.max_health);
        if !rt || stage == 0 {
            if let Some(original) = original {
                commands
                    .entity(e)
                    .insert(MeshMaterial3d(original.material.clone()))
                    .remove::<MeshMaterial3d<Cracked>>()
                    .remove::<Original>();
            }
            continue;
        }
        if original.is_some_and(|o| o.stage == stage) {
            continue;
        }
        let source = original
            .map(|o| o.material.clone())
            .or_else(|| standard.map(|m| m.0.clone()));
        let Some(source) = source else {
            continue;
        };
        let Some(mut material) = base.get(&source).cloned() else {
            continue;
        };
        let Some(texture) = textures.stages.get(stage) else {
            continue;
        };
        material.opaque_render_method = bevy::material::OpaqueRendererMethod::Deferred;
        let variant = ((t.translation.x as i32 as u32).wrapping_mul(73)
            ^ (t.translation.y as i32 as u32).wrapping_mul(139))
            % 3;
        let generated = materials.add(Cracked {
            base: material,
            extension: CrackExtension {
                texture: texture.clone(),
                parameters: Vec4::new(
                    variant as f32,
                    if *kind == BlockType::Cactus { 0.9 } else { 1. },
                    0.,
                    0.,
                ),
            },
        });
        if let Some(old) = owned.0.insert(e, generated.clone()) {
            materials.remove(old.id());
        }
        commands
            .entity(e)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert((
                MeshMaterial3d(generated.clone()),
                Original {
                    material: source,
                    stage,
                },
            ));
    }
}
