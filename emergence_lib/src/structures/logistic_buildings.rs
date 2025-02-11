//! Logic for buildings that move items around.

use bevy::prelude::*;

use crate::{
    crafting::{
        inventories::{InputInventory, OutputInventory},
        item_tags::ItemKind,
        recipe::RecipeInput,
    },
    geometry::{Facing, Height, MapGeometry, VoxelPos},
    items::item_manifest::ItemManifest,
    litter::Litter,
    signals::{Emitter, SignalStrength, SignalType},
    simulation::SimulationSet,
    water::WaterDepth,
};

use super::Footprint;

/// A building that spits out items.
#[derive(Component)]
pub(crate) struct ReleasesItems;

/// A building that takes in items.
#[derive(Component)]
pub(crate) struct AbsorbsItems;

/// Logic that controls how items are moved around by structures.
pub(super) struct LogisticsPlugin;

impl Plugin for LogisticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (release_items, absorb_items, logistic_buildings_signals)
                .in_set(SimulationSet)
                .in_schedule(CoreSchedule::FixedUpdate),
        );
    }
}

/// Causes buildings that emit items to place them in the litter in front of them.
fn release_items(
    mut structure_query: Query<(&VoxelPos, &Facing, &mut InputInventory), With<ReleasesItems>>,
    mut litter_query: Query<&mut Litter>,
    item_manifest: Res<ItemManifest>,
    map_geometry: Res<MapGeometry>,
) {
    for (structure_pos, structure_facing, mut input_inventory) in structure_query.iter_mut() {
        let voxel_pos = structure_pos.neighbor(structure_facing.direction);

        let litter_entity = map_geometry.get_terrain(voxel_pos.hex).unwrap();
        let mut litter = litter_query.get_mut(litter_entity).unwrap();

        let cloned_inventory = input_inventory.clone();
        for item_slot in cloned_inventory.iter() {
            let item_count = item_slot.item_count();

            if litter
                .contents
                .add_item_all_or_nothing(&item_count, &item_manifest)
                .is_ok()
            {
                let recipe_input = RecipeInput::Exact(vec![item_count]);
                input_inventory
                    .consume_items(&recipe_input, &item_manifest)
                    .unwrap();
            }
        }
    }
}

/// Absorb litter into the inventory of buildings that absorb items.
fn absorb_items(
    mut structure_query: Query<(&VoxelPos, &Footprint, &mut OutputInventory), With<AbsorbsItems>>,
    mut litter_query: Query<&mut Litter>,
    item_manifest: Res<ItemManifest>,
    water_depth_query: Query<&WaterDepth>,
    map_geometry: Res<MapGeometry>,
) {
    for (&voxel_pos, footprint, mut output_inventory) in structure_query.iter_mut() {
        output_inventory.clear_empty_slots();

        if output_inventory.is_full() {
            continue;
        }

        let litter_entity = map_geometry.get_terrain(voxel_pos.hex).unwrap();
        let mut litter = litter_query.get_mut(litter_entity).unwrap();

        let on_ground = litter.contents.clone();

        for item_slot in on_ground.iter() {
            let item_count = item_slot.item_count();

            if output_inventory
                .add_item_all_or_nothing(&item_count, &item_manifest)
                .is_ok()
            {
                litter.contents.try_remove_item(&item_count).unwrap();
            }
        }

        // Only absorb floating items if the structure is tall enough.
        let terrain_entity = map_geometry.get_terrain(voxel_pos.hex).unwrap();
        let water_depth = water_depth_query.get(terrain_entity).unwrap();

        if Height::from(footprint.max_height()) > water_depth.surface_water_depth() {
            let floating = litter.contents.clone();
            for item_slot in floating.iter() {
                let item_count = item_slot.item_count();

                if output_inventory
                    .add_item_all_or_nothing(&item_count, &item_manifest)
                    .is_ok()
                {
                    litter.contents.try_remove_item(&item_count).unwrap();
                }
            }
        }
    }
}

/// Sets the emitters for logistic buildings.
fn logistic_buildings_signals(
    mut release_query: Query<
        (&mut Emitter, &mut InputInventory),
        (With<ReleasesItems>, Without<AbsorbsItems>),
    >,
    mut absorb_query: Query<
        (&mut Emitter, &mut OutputInventory),
        (With<AbsorbsItems>, Without<ReleasesItems>),
    >,
) {
    /// Controls how strong the signal is for logistic buildings.
    const LOGISTIC_SIGNAL_STRENGTH: f32 = 10.;

    let signal_strength = SignalStrength::new(LOGISTIC_SIGNAL_STRENGTH);

    for (mut emitter, input_inventory) in release_query.iter_mut() {
        emitter.signals.clear();
        for item_slot in input_inventory.iter() {
            if !item_slot.is_full() {
                let item_kind = match *input_inventory {
                    InputInventory::Exact { .. } => ItemKind::Single(item_slot.item_id()),
                    InputInventory::Tagged { tag, .. } => ItemKind::Tag(tag),
                };

                // This should be a Pull signal, rather than a Stores signal to
                // ensure that goods can be continuously harvested and shipped.
                let signal_type: SignalType = SignalType::Pull(item_kind);
                emitter.signals.push((signal_type, signal_strength));
            }
        }
    }

    for (mut emitter, output_inventory) in absorb_query.iter_mut() {
        emitter.signals.clear();
        for item_slot in output_inventory.iter() {
            if !item_slot.is_full() {
                let item_kind = ItemKind::Single(item_slot.item_id());

                // This should be a Push signal, rather than a Contains signal to
                // ensure that the flow of goods becomes unblocked.
                let signal_type: SignalType = SignalType::Push(item_kind);
                emitter.signals.push((signal_type, signal_strength));
            }
        }
    }
}
