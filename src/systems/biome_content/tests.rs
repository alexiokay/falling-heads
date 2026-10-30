use super::*;
use crate::components::block::{BiomeType as B, BlockType};
#[test]
fn geodes_have_a_hollow_core_and_two_material_rings() {
    let mut counts = [0; 3];
    for y in -100..100 {
        for x in -100..100 {
            match terrain::geode(x, y, B::Underground, 921) {
                Some(None) => counts[0] += 1,
                Some(Some(Tile::Calcite)) => counts[1] += 1,
                Some(Some(Tile::Amethyst)) => counts[2] += 1,
                _ => {}
            }
        }
    }
    assert!(counts.iter().all(|n| *n > 0));
    assert_eq!(terrain::geode(0, 0, B::Forest, 921), None);
}
#[test]
fn geology_preserves_minerals_and_plants_have_support() {
    for b in [
        B::Plains,
        B::Forest,
        B::Desert,
        B::Mountains,
        B::Nether,
        B::End,
        B::Iceland,
    ] {
        for x in -32..32 {
            for y in -32..32 {
                assert_eq!(terrain::solid(x, y, b, 921, BlockType::Diamond), None);
                if let Some(tile) = terrain::decoration(x, y, b, 921) {
                    assert!(terrain::open(x, y, b, 921));
                    assert!(!terrain::open(
                        x,
                        y + if tile == Tile::DripstoneDown { 1 } else { -1 },
                        b,
                        921
                    ));
                }
            }
        }
    }
}
#[test]
fn fire_materials_are_not_inferred_from_sprite_shape() {
    assert!(!Tile::Crystal.combustible());
    assert!(!Tile::DripstoneUp.combustible());
    assert!(Tile::Hay.combustible());
    assert!(Tile::Berries.combustible());
    assert_eq!(Tile::PackedIce.base(), BlockType::Ice);
    assert_eq!(Tile::PowderSnow.base(), BlockType::Snow);
}
#[test]
fn homing_cannot_snap_to_a_target_behind_the_projectile() {
    let v = projectiles::turn(Vec2::new(150., 0.), Vec2::new(-50., 5.), 0.085);
    assert!((v.length() - 150.).abs() < 0.001);
    assert!(v.y.atan2(v.x).abs() <= 0.0851);
    assert!(v.x > 149.);
}
#[test]
fn swept_projectile_detects_wall_even_when_frame_crosses_multiple_blocks() {
    let hit = crate::systems::projectile_system::segment_box(
        Vec2::ZERO,
        Vec2::new(900., 0.),
        Vec2::new(128., 0.),
        Vec2::splat(34.),
    )
    .unwrap();
    assert!((hit - 94. / 900.).abs() < 0.0001);
}
#[test]
fn added_species_have_distinct_pack_faces_and_health() {
    use crate::components::enemy::EnemyBehavior;
    let names: std::collections::HashSet<_> = enemies::SPECIES.iter().map(|s| s.asset()).collect();
    assert_eq!(names.len(), 10);
    assert!(enemies::SPECIES.iter().all(|s| s.max_health() > 0));
}

#[test]
fn farms_and_fossils_exist_on_real_seeded_terrain() {
    for (biome, wanted) in [(B::Plains, Tile::Hay), (B::Desert, Tile::Bone)] {
        let mut found = false;
        for y in -80..80 {
            for x in -120..120 {
                if terrain::feature(x, y, biome, 921) == Some(wanted) {
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        assert!(found, "Missing grounded {wanted:?} features");
    }
}
#[test]
fn streamed_feature_cells_have_support_or_a_short_connected_footing() {
    for y in -20..20 {
        for x in -48..48 {
            if let Some(tile) = terrain::feature(x, y, B::Plains, 921)
                && (tile == Tile::Wheat || tile == Tile::Hay)
            {
                assert!(
                    !terrain::open(x, y - 1, B::Plains, 921)
                        || terrain::feature(x, y - 1, B::Plains, 921).is_some()
                );
            }
        }
    }
}
