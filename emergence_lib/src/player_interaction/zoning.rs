//! Selecting structures to place, and then setting tiles as those structures.

use bevy::prelude::*;
use bevy_ecs_tilemap::tiles::TilePos;
use leafwing_input_manager::prelude::*;

use crate::organisms::OrganismType;

use super::{cursor::CursorTilePos, tile_selection::SelectedTiles, InteractionSystem};

/// Logic and resources for structure selection and placement.
pub struct ZoningPlugin;

impl Plugin for ZoningPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedStructure>()
            .init_resource::<ActionState<ZoningAction>>()
            .insert_resource(ZoningAction::default_input_map())
            .add_plugin(InputManagerPlugin::<ZoningAction>::default())
            .add_system(
                set_selected_structure
                    .label(InteractionSystem::SelectStructure)
                    .after(InteractionSystem::ComputeCursorPos),
            )
            .add_system(zone_selected_tiles.after(InteractionSystem::SelectTiles))
            .add_system(display_selected_structure.after(InteractionSystem::SelectStructure));
    }
}

/// Tracks which structure the player has selected, if any
#[derive(Resource, Default)]
pub struct SelectedStructure {
    /// Which structure is selected
    // FIXME: should only be able to store structures. Units should be excluded.
    pub maybe_structure: Option<OrganismType>,
}

/// Actions that the player can take to select and place structures
#[derive(Actionlike, Clone, PartialEq, Debug)]
pub enum ZoningAction {
    /// Selects the structure on the tile under the player's cursor.
    ///
    /// If there is no structure there, the player's selection is cleared.
    Pipette,
    /// Clears the current structure selection.
    ClearSelection,
    /// Sets the zoning of all currently selected tiles to the currently selected structure.
    ///
    /// If no structure is selected, any zoning will be removed.
    Zone,
}

impl ZoningAction {
    /// The default keybindings
    fn default_input_map() -> InputMap<ZoningAction> {
        InputMap::new([
            (KeyCode::Q, ZoningAction::Pipette),
            (KeyCode::Space, ZoningAction::Zone),
            (KeyCode::Back, ZoningAction::ClearSelection),
            (KeyCode::Delete, ZoningAction::ClearSelection),
        ])
    }
}

/// Sets which structure the player has selected.
fn set_selected_structure(
    zoning_actions: Res<ActionState<ZoningAction>>,
    mut selected_structure: ResMut<SelectedStructure>,
    cursor_pos: Res<CursorTilePos>,
    structure_query: Query<(&TilePos, &OrganismType)>,
) {
    // Clearing should take priority over selecting a new item (on the same frame)
    if zoning_actions.just_pressed(ZoningAction::ClearSelection) {
        selected_structure.maybe_structure = None;
    } else if zoning_actions.just_pressed(ZoningAction::Pipette) {
        // PERF: this needs to use an index, rather than a linear time search
        let mut structure_under_cursor = None;
        for (&tile_pos, organism_type) in structure_query.iter() {
            if Some(tile_pos) == cursor_pos.maybe_tile_pos() {
                structure_under_cursor = Some(organism_type.clone());
                break;
            }
        }

        selected_structure.maybe_structure = structure_under_cursor;
    }
}

/// Shows which structure the player has selected.
fn display_selected_structure(selected_structure: Res<SelectedStructure>) {
    if selected_structure.is_changed() {
        let selected_structure = &selected_structure.maybe_structure;
        info!("Currently selected: {selected_structure:?}");
    }
}

/// Applies zoning to an area
fn zone_selected_tiles(
    zoning_actions: Res<ActionState<ZoningAction>>,
    selected_structure: Res<SelectedStructure>,
    selected_tiles: Res<SelectedTiles>,
) {
    if zoning_actions.pressed(ZoningAction::Zone) {
        // TODO: actually zone tiles
        for &tile in selected_tiles.selection() {
            let selected_structure = &selected_structure.maybe_structure;
            info!("Zoning: {tile:?} to {selected_structure:?}.");
        }
    }
}