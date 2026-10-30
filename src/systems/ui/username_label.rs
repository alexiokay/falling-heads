use crate::components::username_label::UsernameLabel;
use bevy::prelude::*;

pub struct UsernameLabelPlugin;

impl Plugin for UsernameLabelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            update_username_labels
                .after(TransformSystems::Propagate)
                .before(bevy::camera::visibility::VisibilitySystems::CheckVisibility),
        );
    }
}

/// Update username label positions to follow their owner entities
/// Projects the final owner AND camera transforms from the same render frame.
fn update_username_labels(
    mut commands: Commands,
    mut label_query: Query<(Entity, &UsernameLabel, &mut Transform, &mut GlobalTransform)>,
    owner_query: Query<&GlobalTransform, Without<UsernameLabel>>,
    camera_query: Query<(&Camera, &GlobalTransform), (With<Camera3d>, Without<UsernameLabel>)>,
    windows: Query<&Window>,
) {
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let Ok(window) = windows.single() else {
        return;
    };
    let window_width = window.width();
    let window_height = window.height();

    for (label_entity, username_label, mut label_transform, mut label_global) in
        label_query.iter_mut()
    {
        if let Ok(owner_transform) = owner_query.get(username_label.owner) {
            let world_pos =
                owner_transform.translation() + Vec3::new(0.0, username_label.offset_y, 0.0);

            if let Ok(screen_pos) = camera.world_to_viewport(camera_transform, world_pos) {
                label_transform.translation.x = (screen_pos.x - (window_width / 2.0)).round();
                label_transform.translation.y = ((window_height / 2.0) - screen_pos.y).round();
                label_transform.translation.z = 0.0;
            } else {
                label_transform.translation = Vec3::new(10000.0, 10000.0, 0.0);
            }
            // Projection happens after propagation: publish the root transform for this render frame.
            *label_global = GlobalTransform::from(*label_transform);
        } else {
            commands.entity(label_entity).despawn();
        }
    }
}

/// A screen-space label anchored to a 3D owner through the current camera.
pub fn spawn_username_label(
    commands: &mut Commands,
    owner_entity: Entity,
    _owner_position: Vec3,
    username: String,
    offset_y: f32,
    _meshes: &mut Assets<Mesh>,
    _materials: &mut Assets<StandardMaterial>,
    _asset_server: &AssetServer,
) {
    // The separate 2D camera renders the projected label.
    // Colors cycle based on username to make each viewer unique
    let username_hash = username
        .chars()
        .fold(0u32, |acc, c| acc.wrapping_add(c as u32));
    let hue = (username_hash % 360) as f32;

    let text_color = Color::hsl(hue, 0.9, 0.7); // Bright, vibrant color

    commands.spawn((
        UsernameLabel {
            username: username.clone(),
            owner: owner_entity,
            offset_y,
        },
        Text2d::new(username.clone()),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(text_color),
        Transform::from_xyz(0.0, 0.0, 0.0), // Will be updated by update_username_labels
    ));

    info!(
        "✨ Username label '{}' spawned for entity {:?}",
        username, owner_entity
    );
}
