use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy::window::PrimaryWindow;
use bimap::BiHashMap;
use bimap::Overwritten;

use crate::screen::playing::GameState;
use crate::screen::Screen;

/// Width of a tile.
pub const TILE_WIDTH: f32 = 256.0;
/// Half height of a tile surface.
pub const TILE_HALF_HEIGHT: f32 = TILE_WIDTH / 4.0 + 26.0;
/// A single right direction unit in the isometric world.
pub const RIGHT_DIR: Vec2 = Vec2::new(TILE_WIDTH / 2.0, -TILE_HALF_HEIGHT);
/// A single down direction unit in the isometric world.
pub const DOWN_DIR: Vec2 = Vec2::new(-TILE_WIDTH / 2.0, -TILE_HALF_HEIGHT);

/// Z-depth of a single layer.
pub const LAYER_DEPTH: f32 = 10.0;

/// Convert tile coordinate to world translation.
pub fn tile_coord_translation(x: f32, y: f32, layer: f32) -> Vec3 {
    let mut translation = RIGHT_DIR.xyy() * x;
    translation += DOWN_DIR.xyy() * y;
    translation.z = translation.z * -0.001 + layer * LAYER_DEPTH;

    translation
}

pub struct TileMapPlugin;

impl Plugin for TileMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TileMap>()
            .init_resource::<TileSet>()
            .init_resource::<PickedTile>()
            .init_resource::<PickedPoint>()
            .add_systems(PreStartup, load_tiles)
            .add_systems(
                Update,
                (find_picked_point, pick_tile)
                    .run_if(in_state(Screen::Playing).and_then(in_state(GameState::Resumed))),
            );
    }
}

#[derive(Resource, Debug, Default)]
pub struct TileMap {
    size: IVec2,
    map: BiHashMap<IVec2, Entity>,
}

/// movement directions on tilemap
pub const NORTH: IVec2 = IVec2::Y;
pub const EAST: IVec2 = IVec2::X;
pub const SOUTH: IVec2 = IVec2 { y: -1, x: 0 };
pub const WEST: IVec2 = IVec2 { x: -1, y: 0 };
pub const NORTHEAST: IVec2 = NORTH.wrapping_add(EAST);
pub const SOUTHEAST: IVec2 = SOUTH.wrapping_add(EAST);
pub const NORTHWEST: IVec2 = NORTH.wrapping_add(WEST);
pub const SOUTHWEST: IVec2 = SOUTH.wrapping_add(WEST);

/// Four directional movement in straight lines like a rook
pub const ROOK_MOVES: [IVec2; 4] = [NORTH, EAST, SOUTH, WEST];

/// Eight directional movement like a king
pub const KING_MOVES: [IVec2; 8] = [
    NORTH, NORTHEAST, EAST, SOUTHEAST, SOUTH, SOUTHWEST, WEST, NORTHWEST,
];

impl TileMap {
    pub fn new(size: IVec2) -> TileMap {
        assert!(IVec2::ZERO.cmplt(size).all());
        TileMap {
            size,
            map: BiHashMap::default(),
        }
    }

    pub fn bounds(&self) -> IRect {
        IRect::from_corners(IVec2::ZERO, self.size - 1)
    }

    /// get entity at position
    pub fn get(&self, position: IVec2) -> Option<Entity> {
        self.map.get_by_left(&position).copied()
    }

    /// find entity's position in map
    pub fn locate(&self, entity: Entity) -> Option<IVec2> {
        self.map.get_by_right(&entity).copied()
    }

    /// place entity at map position, will move entity if already in map.
    /// will overwrite any existing entity at the position
    pub fn set(&mut self, position: IVec2, entity: Entity) -> Overwritten<IVec2, Entity> {
        self.map.insert(position, entity)
    }

    /// remove entity from map at position
    pub fn remove(&mut self, position: IVec2) -> Option<Entity> {
        self.map.remove_by_left(&position).map(|(_, entity)| entity)
    }

    /// remove entity from map
    pub fn remove_entity(&mut self, entity: Entity) -> Option<IVec2> {
        self.map
            .remove_by_right(&entity)
            .map(|(position, _)| position)
    }

    pub fn get_neighbouring_positions_rook<'a>(
        &'a self,
        position: IVec2,
    ) -> impl Iterator<Item = IVec2> + 'a {
        ROOK_MOVES
            .iter()
            .copied()
            .map(move |translation| position + translation)
            .filter(|target| self.bounds().contains(*target))
    }

    pub fn get_neighbouring_positions_king<'a>(
        &'a self,
        position: IVec2,
    ) -> impl Iterator<Item = IVec2> + 'a {
        KING_MOVES
            .iter()
            .copied()
            .map(move |translation| position + translation)
            .filter(|target| self.bounds().contains(*target))
    }
}

#[derive(Resource, Default, Debug)]
pub struct TileSet(HashMap<&'static str, Handle<Image>>);

impl TileSet {
    pub fn insert(&mut self, name: &'static str, handle: Handle<Image>) -> Option<Handle<Image>> {
        self.0.insert(name, handle)
    }

    /// Get cloned image handle.
    ///
    /// # Panic
    ///
    /// For ease of use, unwrap is used to panic if value does not exists for certain key.
    pub fn get(&self, name: &str) -> Handle<Image> {
        self.0.get(name).unwrap().clone()
    }
}

fn load_tiles(asset_server: Res<AssetServer>, mut tile_set: ResMut<TileSet>) {
    const TILES: &[&str] = &[
        "grassblock",
        "gravelblock",
        "waterblock",
        "house1",
        "werewolf",
        "human",
    ];

    for &tile in TILES {
        info!("Loading tile: {}", tile);
        tile_set.insert(tile, asset_server.load(format!("tiles/{}.png", tile)));
    }
}

#[derive(Resource, Default, Debug)]
pub struct PickedTile(pub Vec<Entity>);

#[derive(Resource, Default)]
pub struct PickedPoint(pub Option<Vec2>);

pub fn find_picked_point(
    mut picked_point: ResMut<PickedPoint>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
) {
    let (camera, camera_transform) = q_camera.single();
    let window = q_window.single();

    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        picked_point.0 = Some(world_position);
    } else {
        picked_point.0 = None;
    }
}

fn is_point_in_triangle(x: f32, y: f32, w: f32, h: f32) -> bool {
    if x < 0.0 || y < 0.0 {
        return false;
    }
    y <= h - (h / w) * x
}

pub fn pick_tile(
    picked_point: Res<PickedPoint>,
    mut picked_tile: ResMut<PickedTile>,
    tiles_query: Query<(Entity, &GlobalTransform), With<PickableTile>>,
    mut sprite_query: Query<&mut Sprite>,
) {
    for previous in picked_tile.0.drain(..) {
        sprite_query
            .get_mut(previous)
            .map(|mut sprite| sprite.color = Color::WHITE)
            .ok();
    }

    if let Some(point) = picked_point.0 {
        for (e, ..) in tiles_query
            .iter()
            .map(|(e, t)| (e, (point - t.translation().xy()).abs(), t.translation().z))
            .filter(|(_, r, _)| is_point_in_triangle(r.x, r.y, 0.5 * TILE_WIDTH, TILE_HALF_HEIGHT))
        {
            sprite_query
                .get_mut(e)
                .map(|mut sprite| sprite.color = Color::srgb(1., 0., 0.))
                .ok();
            picked_tile.0.push(e);
        }
    }
}

#[derive(Component)]
pub struct PickableTile;
