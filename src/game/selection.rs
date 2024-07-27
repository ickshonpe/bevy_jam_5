use bevy::math::vec2;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy::utils::HashSet;

use crate::game::map::ROOK_MOVES;
use crate::path_finding::find_all_within_distance_unweighted;
use crate::screen::playing::GameState;
use crate::screen::Screen;

use super::deployment::deploy_unit;
use super::map::VillageMap;
use super::picking::PickedTile;

pub struct SelectionPlugin;

impl Plugin for SelectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedTiles>()
            .init_resource::<SelectionMap>()
            .init_resource::<SelectedUnit>()
            .add_event::<SelectionEvent>()
            .add_systems(
                Update,
                (
                    show_selected_tiles.run_if(resource_changed::<SelectedTiles>),
                    set_selected_unit
                        .run_if(in_state(Screen::Playing))
                        .before(deploy_unit),
                    on_selection.after(set_selected_unit),
                    show_movement_range
                        .after(on_selection)
                        .run_if(not(in_state(GameState::Deployment))),
                )
                    .run_if(in_state(Screen::Playing)),
            );
    }
}

/// Current selected unit, can be Player controlled, enemy or a building
#[derive(Resource, Default)]
pub struct SelectedUnit {
    pub entity: Option<Entity>,
}

impl SelectedUnit {
    pub fn set(&mut self, entity: Entity) {
        self.entity = Some(entity);
    }
}

#[derive(Resource, Default)]
pub struct SelectedTiles {
    pub color: Color,
    pub tiles: HashSet<IVec2>,
}

#[derive(Resource, Default)]
pub struct SelectionMap {
    pub tiles: HashMap<IVec2, [Entity; 4]>,
}

#[derive(Component, Copy, Clone, Debug)]
pub enum SelectionEdge {
    North,
    East,
    South,
    West,
}

impl SelectionEdge {
    pub const ALL: [Self; 4] = [Self::North, Self::East, Self::South, Self::West];

    pub fn get_scalar(&self) -> Vec2 {
        match self {
            SelectionEdge::North => Vec2::ONE,
            SelectionEdge::East => vec2(1., -1.),
            SelectionEdge::South => -Vec2::ONE,
            SelectionEdge::West => vec2(-1., 1.),
        }
    }
}

pub fn show_selected_tiles(
    selected_tiles: Res<SelectedTiles>,
    tile_ids: Res<SelectionMap>,
    mut query: Query<(&mut Sprite, &mut Visibility), With<SelectionEdge>>,
) {
    for (_, mut vis) in query.iter_mut() {
        vis.set_if_neq(Visibility::Hidden);
    }

    for &tile in selected_tiles.tiles.iter() {
        let Some(s) = tile_ids.tiles.get(&tile) else {
            continue;
        };
        let neighbours = ROOK_MOVES
            .map(|m| tile + m)
            .map(|n| selected_tiles.tiles.contains(&n));
        for (i, a) in neighbours.into_iter().enumerate() {
            if !a {
                if let Ok((mut sprite, mut vis)) = query.get_mut(s[i]) {
                    sprite.color = selected_tiles.color;
                    *vis = Visibility::Visible;
                }
            }
        }
    }
}

pub fn show_movement_range(
    selected_unit: Res<SelectedUnit>,
    mut selected_tiles: ResMut<SelectedTiles>,
    village_map: Res<VillageMap>,
) {
    if let Some(entity) = selected_unit.entity {
        if let Some(tile) = village_map.object.locate(entity) {
            let tiles = find_all_within_distance_unweighted(tile, 4, |t| {
                village_map.object.get_neighbouring_positions_rook(t)
            });
            selected_tiles.tiles = tiles;
        }
    }
}

#[derive(Event, Debug)]
pub enum SelectionEvent {
    Selected(Entity),
    Deselected(Entity),
}

#[derive(Event)]
pub struct DeselectedUnitEvent(pub Entity);

pub fn set_selected_unit(
    mouse_button: Res<ButtonInput<MouseButton>>,
    picked_tile: Res<PickedTile>,
    village_map: Res<VillageMap>,
    mut selected_unit: ResMut<SelectedUnit>,
    mut selection_event: EventWriter<SelectionEvent>,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        if let Some(tile) = picked_tile.0 {
            if let Some(new_selection) = village_map.object.get(tile) {
                if let Some(previous_selection) = selected_unit.entity {
                    if new_selection == previous_selection {
                        return;
                    }
                    selection_event.send(SelectionEvent::Deselected(previous_selection));
                }
                selection_event.send(SelectionEvent::Selected(new_selection));
                selected_unit.entity = Some(new_selection);
            }
        }
    }
}

#[derive(Component)]
pub struct SelectionMarkerSprite;

pub fn on_selection(
    mut commands: Commands,
    mut selection_events: EventReader<SelectionEvent>,
    query: Query<Entity, With<SelectionMarkerSprite>>,
    asset_server: Res<AssetServer>,
) {
    for selection_event in selection_events.read() {
        match selection_event {
            SelectionEvent::Selected(entity) => {
                if let Some(mut entity_commands) = commands.get_entity(*entity) {
                    entity_commands.with_children(|builder| {
                        builder.spawn((
                            SpriteBundle {
                                sprite: Sprite {
                                    anchor: bevy::sprite::Anchor::BottomCenter,
                                    color: Color::WHITE,
                                    custom_size: Some(Vec2::splat(64.)),
                                    ..Default::default()
                                },
                                texture: asset_server.load("icons/selection_arrow.png"),
                                transform: Transform::from_xyz(0., 100., 0.1),
                                ..Default::default()
                            },
                            SelectionMarkerSprite,
                        ));
                    });
                }
            }
            SelectionEvent::Deselected(_) => {
                for entity in query.iter() {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}
