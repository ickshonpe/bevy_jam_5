use bevy::asset::LoadState;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bimap::BiHashMap;
use bimap::Overwritten;

use self::level_asset::LevelAsset;
use self::level_asset::LevelPlugin;
use self::level_asset::Levels;

pub mod level_asset;

/// Width of a tile.
pub const TILE_SIZE: f32 = 256.0;
/// A single right direction unit in the isometric world.
pub const RIGHT_DIR: Vec2 = Vec2::new(TILE_SIZE / 2.0, -TILE_SIZE / 4.0 - 26.0);
/// A single down direction unit in the isometric world.
pub const DOWN_DIR: Vec2 = Vec2::new(-TILE_SIZE / 2.0, -TILE_SIZE / 4.0 - 26.0);

/// Z-depth of a single layer.
pub const LAYER_DEPTH: f32 = 10.0;

/// Convert tile coordinate to world translation.
pub fn tile_coord_translation(x: f32, y: f32, layer: f32) -> Vec3 {
    let mut translation = Vec3::new(RIGHT_DIR.x, RIGHT_DIR.y, -RIGHT_DIR.y) * x;
    translation += Vec3::new(DOWN_DIR.x, DOWN_DIR.y, -DOWN_DIR.y) * y;
    translation.z += layer * LAYER_DEPTH;

    translation
}

pub struct TileMapPlugin;

impl Plugin for TileMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LevelPlugin)
            .init_resource::<TileMap>()
            .init_resource::<TileSet>()
            .add_systems(PreStartup, load_tiles)
            .add_systems(Update, load_debug_level);
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
pub const WEST: IVec2 = IVec2 { x: -1, y: 0  };
pub const NORTHEAST: IVec2 = NORTH.wrapping_add(EAST);
pub const SOUTHEAST: IVec2 = SOUTH.wrapping_add(EAST);
pub const NORTHWEST: IVec2 = NORTH.wrapping_add(WEST);
pub const SOUTHWEST: IVec2 = SOUTH.wrapping_add(WEST);

/// Four directional movement in straight lines like a rook
pub const ROOK_MOVES: [IVec2; 4] = [NORTH, EAST, SOUTH, WEST];

/// Eight directional movement like a king
pub const KING_MOVES: [IVec2; 8] = [NORTH, NORTHEAST, EAST, SOUTHEAST, SOUTH, SOUTHWEST, WEST, NORTHWEST];

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
    
    pub fn get_neighbouring_positions_rook<'a>(&'a self, position: IVec2) -> impl Iterator<Item = IVec2> + 'a {
        ROOK_MOVES.iter().copied()
        .map(move |translation| position + translation)
        .filter(|target| self.bounds().contains(*target))
    }

    pub fn get_neighbouring_positions_king<'a>(&'a self, position: IVec2) -> impl Iterator<Item = IVec2> + 'a {
        KING_MOVES.iter().copied()
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
    tile_set.insert("block_grey", asset_server.load("tiles/block_grey.png"));
    tile_set.insert("block_blue", asset_server.load("tiles/block_blue.png"));
    tile_set.insert("block_green", asset_server.load("tiles/block_green.png"));
    tile_set.insert("block_orange", asset_server.load("tiles/block_orange.png"));
}

fn load_debug_level(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    levels: Res<Levels>,
    level_assets: Res<Assets<LevelAsset>>,
    tile_set: Res<TileSet>,
    mut loaded: Local<bool>,
) {
    if *loaded {
        return;
    }

    let Some(level_handle) = levels.0.get("debug_level") else {
        warn!("No debug level found..");
        return;
    };
    let Some(load_state) = asset_server.get_load_state(level_handle) else {
        warn!("No load state for level: {level_handle:?}..");
        return;
    };

    if let LoadState::Loaded = load_state {
        let debug_level = level_assets.get(level_handle).unwrap();
        println!("loading level: {}", debug_level.name);

        let start_translation = Vec3::new(0.0, 1000.0, 0.0);

        for (layer, tiles) in debug_level.tiles.iter().enumerate() {
            for (i, tile) in tiles.iter().enumerate() {
                let x = (i % debug_level.size) as f32;
                let y = (i / debug_level.size) as f32;
                let translation = start_translation + tile_coord_translation(x, y, layer as f32);

                commands.spawn(SpriteBundle {
                    texture: tile_set.get(tile),
                    transform: Transform::from_translation(translation),
                    ..default()
                });
            }
        }
        *loaded = true;
    }
}
