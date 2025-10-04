use bevy::prelude::*;
use crate::resources::inventory::Inventory;
use crate::states::GameState;

pub struct UiPlugin;

#[derive(Component)]
struct InventoryText;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::Playing), setup_ui)
            .add_systems(Update, update_inventory_display.run_if(in_state(GameState::Playing)));

        info!("UiPlugin loaded");
    }
}

fn setup_ui(mut commands: Commands) {
    // Spawn UI root
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|parent| {
            // Inventory display
            parent.spawn((
                Text::new("Resources: "),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                InventoryText,
            ));
        });

    info!("UI spawned");
}

fn update_inventory_display(
    inventory: Res<Inventory>,
    mut query: Query<&mut Text, With<InventoryText>>,
) {
    if !inventory.is_changed() {
        return;
    }

    for mut text in query.iter_mut() {
        **text = format!(
            "Resources: Coal:{} Iron:{} Gold:{} Diamond:{} Emerald:{} Redstone:{}",
            inventory.coal,
            inventory.iron,
            inventory.gold,
            inventory.diamond,
            inventory.emerald,
            inventory.redstone
        );
    }
}
