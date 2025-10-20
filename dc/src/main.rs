use color_eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::{prelude::ThreadRng, Rng};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Table, Row, Cell},
    Terminal,
};
use std::io;
use std::collections::HashMap;
use std::collections::HashSet;

// --- Entity Constants and Structs ---
const MAX_ZOMBIES: usize = 25;
const ZOMBIE_SPAWN_CHANCE: f32 = 0.5;
const ZOMBIE_SPAWN_RADIUS: i32 = 8;

#[derive(Clone, Copy, Debug)]
struct Zombie {
    x: i32,
    y: i32,
    hp: i32, // <-- ADDED: Health Points for the zombie
}

// --- Enums and Structs ---

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TileType {
    Wall,
    Floor,
    Zombie,
    Foliage,
    Car,
    Resource,
    Building,
    Mall,
    Weapon, 
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Shade {
    Dark,
    Dim,
    Lit,
    Bright,
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl Rect {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Rect { x1, y1, x2, y2 }
    }

    pub fn center(&self) -> (i32, i32) {
        ((self.x1 + self.x2) / 2, (self.y1 + self.y2) / 2)
    }

    pub fn create_room(&self, rng: &mut ThreadRng) -> Self {
        let mut room = *self;
        room.x1 += rng.gen_range(1..=4);
        room.y1 += rng.gen_range(1..=4);
        room.x2 -= rng.gen_range(1..=4);
        room.y2 -= rng.gen_range(1..=4);
        if room.x1 >= room.x2 || room.y1 >= room.y2 {
            *self
        } else {
            room
        }
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x1 <= other.x2 && self.x2 >= other.x1 && self.y1 <= other.y2 && self.y2 >= other.y1
    }
}

#[derive(Clone, Debug)]
pub struct Map {
    pub tiles: Vec<TileType>,
    pub width: usize,
    pub height: usize,
    pub rooms: Vec<Rect>,
}

impl Map {
    pub fn new(width: usize, height: usize) -> Self {
        let tiles = vec![TileType::Wall; width * height];
        Map { tiles, width, height, rooms: vec![] }
    }

    pub fn xy_idx(&self, x: i32, y: i32) -> usize {
        ((y * self.width as i32) + x) as usize
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32
    }

    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        // Walkability check simplified, movement handler checks for entity collision
        self.in_bounds(x, y) && self.tiles[self.xy_idx(x, y)] != TileType::Wall
    }

    pub fn apply_room(&mut self, room: &Rect, _rng: &mut ThreadRng) {
        for y in room.y1..=room.y2 {
            for x in room.x1..=room.x2 {
                if self.in_bounds(x, y) {
                    let idx = self.xy_idx(x, y);
                    self.tiles[idx] = TileType::Floor;
                }
            }
        }
    }

    pub fn apply_h_tunnel(&mut self, x1: i32, x2: i32, y: i32) {
        let min_x = std::cmp::min(x1, x2);
        let max_x = std::cmp::max(x1, x2);
        for x in min_x..=max_x {
            if self.in_bounds(x, y) {
                let idx = self.xy_idx(x, y);
                self.tiles[idx] = TileType::Floor;
            }
        }
    }

    pub fn apply_v_tunnel(&mut self, y1: i32, y2: i32, x: i32) {
        let min_y = std::cmp::min(y1, y2);
        let max_y = std::cmp::max(y1, y2);
        for y in min_y..=max_y {
            if self.in_bounds(x, y) {
                let idx = self.xy_idx(x, y);
                self.tiles[idx] = TileType::Floor;
            }
        }
    }

    pub fn generate_bsp(&mut self, rng: &mut ThreadRng) {
        // FIX: Removed duplicate definition. This is the single, correct version.
        println!("BSP gen firing—target 10-20 rooms");
        let mut rects = vec![Rect::new(1, 1, self.width as i32 - 2, self.height as i32 - 2)];

        while !rects.is_empty() && self.rooms.len() < 20 {
            let idx = rng.gen_range(0..rects.len());
            let current = rects.swap_remove(idx);

            let room = current.create_room(rng);
            let valid = !self.rooms.iter().any(|r| room.intersects(r));
            if valid {
                self.apply_room(&room, rng); 
                self.rooms.push(room);       
            }

            if current.x2 - current.x1 >= 12 && current.y2 - current.y1 >= 12 {
                let h_split = rng.gen_bool(0.5);
                let split_dim_size = if h_split { current.y2 - current.y1 } else { current.x2 - current.x1 };
                if split_dim_size > 8 {
                    let split = if h_split {
                        rng.gen_range(current.y1 + 4..current.y2 - 4)
                    } else {
                        rng.gen_range(current.x1 + 4..current.x2 - 4)
                    };

                    let child1 = if h_split {
                        Rect::new(current.x1, current.y1, current.x2, split)
                    } else {
                        Rect::new(current.x1, current.y1, split, current.y2)
                    };

                    let child2 = if h_split {
                        Rect::new(current.x1, split, current.x2, current.y2)
                    } else {
                        Rect::new(split, current.y1, current.x2, current.y2)
                    };

                    if child1.x2 - child1.x1 >= 6 && child1.y2 - child1.y1 >= 6 {
                        rects.push(child1);
                    }
                    if child2.x2 - child2.x1 >= 6 && child2.y2 - child2.y1 >= 6 {
                        rects.push(child2);
                    }
                }
            }
        }

        println!("Gen done: {} rooms carved", self.rooms.len());

        for i in 1..self.rooms.len() {
            let (prev_x, prev_y) = self.rooms[i - 1].center();
            let (curr_x, curr_y) = self.rooms[i].center();
            
            if rng.gen_bool(0.5) {
                self.apply_h_tunnel(prev_x, curr_x, prev_y);
                self.apply_v_tunnel(curr_y, prev_y, curr_x);
            } else {
                self.apply_v_tunnel(prev_y, curr_y, prev_x);
                self.apply_h_tunnel(curr_x, prev_x, curr_y);
            }

            // Tunnel Zombie (Guaranteed early encounter)
            if i == 1 {
                let (z_x, z_y) = if prev_x != curr_x {
                    ((prev_x + curr_x) / 2, prev_y)
                } else {
                    (prev_x, (prev_y + curr_y) / 2)
                };
                if self.in_bounds(z_x, z_y) {
                    let idx = self.xy_idx(z_x, z_y);
                    if self.tiles[idx] == TileType::Floor {
                         // Note: We use the TileType::Zombie *only* for the static map display logic.
                         self.tiles[idx] = TileType::Zombie; 
                    }
                }
            }
        }
        
        // --- Static Population & Feature Generation ---
        let first_room_rect = self.rooms.first().copied();

        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                let idx = self.xy_idx(x, y);
                
                if self.tiles[idx] != TileType::Floor {
                    continue;
                }

                // Determine if this tile is within the safe starting room
                let is_in_start_room = if let Some(rect) = first_room_rect {
                    x >= rect.x1 && x <= rect.x2 && y >= rect.y1 && y <= rect.y2
                } else {
                    false
                };
                
                // 1. Place FEATURES (Foliage, Car, etc.)
                if rng.gen_bool(0.08) { 
                    match rng.gen_range(0..25) { 
                        0..=10 => self.tiles[idx] = TileType::Foliage,
                        11..=17 => self.tiles[idx] = TileType::Car,
                        18..=20 => self.tiles[idx] = TileType::Resource,
                        21..=23 => self.tiles[idx] = TileType::Building,
                        24 => self.tiles[idx] = TileType::Weapon, 
                        _ => {}
                    }
                }
            }
        }

        // Apply Mall *ONLY* to the last room.
        if self.rooms.len() > 1 {
            if let Some(last_room) = self.rooms.last() {
                for y in last_room.y1..=last_room.y2 {
                    for x in last_room.x1..=last_room.x2 {
                        if self.in_bounds(x, y) {
                            let idx = self.xy_idx(x, y);
                            self.tiles[idx] = TileType::Mall;
                        }
                    }
                }
            }
        }
    }

    pub fn compute_fov(&self, px: i32, py: i32, radius: i32) -> HashMap<(i32, i32), Shade> {
        let mut visible: HashMap<(i32, i32), Shade> = HashMap::new();
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy > radius * radius { continue; }
                let tx = px + dx;
                let ty = py + dy;

                if !self.in_bounds(tx, ty) { continue; }

                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                let blocked = self.tiles[self.xy_idx(tx, ty)] == TileType::Wall;

                let shade = if blocked {
                    Shade::Dark
                } else if dist > (radius as f32) * 0.7f32 {
                    Shade::Dim
                } else if dist > (radius as f32) * 0.3f32 {
                    Shade::Lit
                } else {
                    Shade::Bright
                };
                visible.insert((tx, ty), shade);
            }
        }
        visible
    }
}

// --- Game State (Restored and Zombie List Added) ---

struct GameState {
    player_x: i32,
    player_y: i32,
    health: i32, 
    thirst: i32, 
    hunger: i32,
    fatigue: i32,
    ammo: i32,   
    inventory: Vec<String>,
    message_log: Vec<String>,
    zombies: Vec<Zombie>, // ADDED: List of all active zombies
}

impl GameState {
    fn new() -> Self {
        Self {
            player_x: 0,
            player_y: 0,
            health: 100,
            thirst: 100,
            hunger: 100,
            fatigue: 100,
            ammo: 10,
            inventory: vec!["rusty knife".to_string()],
            message_log: vec!["Radio crackles: 'Help, the horde's at the mall!'".to_string()],
            zombies: vec![],
        }
    }
}

// --- Zombie Logic (Adapted) ---

// RENAMED and UPDATED for HP-based combat
fn handle_attack(map: &mut Map, state: &mut GameState, tx: i32, ty: i32, rng: &mut ThreadRng) {
    let zombie_index = state.zombies.iter().position(|z| z.x == tx && z.y == ty);

    if zombie_index.is_none() {
        state.message_log.push("Attacked empty space.".to_string());
        return;
    }

    state.fatigue = state.fatigue.saturating_sub(5);
    state.hunger = state.hunger.saturating_sub(1);
    state.thirst = state.thirst.saturating_sub(1);

    // --- Weapon and Damage Calculation ---
    let weapon_used: &str;
    let base_hit_chance: f32;
    let damage_range: std::ops::RangeInclusive<i32>;
    let ammo_cost: i32;

    if state.inventory.contains(&"gun".to_string()) && state.ammo > 0 {
        base_hit_chance = 0.95;
        damage_range = 10..=15;
        weapon_used = "Gun";
        ammo_cost = 1;
    } else {
        base_hit_chance = 0.50;
        damage_range = 1..=3;
        weapon_used = "Rusty Knife";
        ammo_cost = 0;
    }

    // --- Fatigue Modifier ---
    // Fatigue penalty: Max 20% penalty at 0 Fatigue.
    let fatigue_penalty = (100 - state.fatigue).max(0) as f32 / 100.0 * 0.2;
    let total_hit_chance = (base_hit_chance - fatigue_penalty).max(0.0);
    
    let zombie_hp_before = state.zombies[zombie_index.unwrap()].hp;

    if rng.gen_bool(total_hit_chance as f64) {
        let damage = rng.gen_range(damage_range);
        let zombie = &mut state.zombies[zombie_index.unwrap()];
        zombie.hp = zombie.hp.saturating_sub(damage);
        
        state.message_log.push(format!("{} HIT! Damage: {}. Zombie HP: {} -> {}. (Chance: {:.0}%)", 
            weapon_used, damage, zombie_hp_before, zombie.hp, total_hit_chance * 100.0));
        
        if ammo_cost > 0 {
            state.ammo = state.ammo.saturating_sub(ammo_cost);
        }

        // --- Victory Check ---
        if zombie.hp <= 0 {
            state.message_log.push(format!("Zombie dispatched by final {} blow!", weapon_used));
            state.zombies.remove(zombie_index.unwrap());
        }

    } else {
        state.message_log.push(format!("{} MISSED! (Chance: {:.0}%)", 
            weapon_used, total_hit_chance * 100.0));
        
        // --- Zombie Counter-Attack ---
        state.health = state.health.saturating_sub(1); // ADJUSTED DAMAGE: 1 HP
        state.message_log.push("Zombie counter-attacks! (-1 Health)".to_string()); // ADJUSTED MESSAGE
    }
    
    while state.message_log.len() > 10 { state.message_log.remove(0); }
}

// FIX: Corrected function signature to use &Map and &mut GameState
fn spawn_random_zombies(
    rng: &mut ThreadRng,
    game_map: &Map,
    state: &mut GameState,
) {
    if state.zombies.len() >= MAX_ZOMBIES {
        return;
    }

    // Lower spawn probability for fewer zombies
    if rng.gen_range(0..100) < 10 { // 10% chance per player move
        let mut attempts = 0;
        while attempts < 10 {
            // Spawn near player within a radius, not just anywhere
            let x = state.player_x + rng.gen_range(-ZOMBIE_SPAWN_RADIUS..=ZOMBIE_SPAWN_RADIUS);
            let y = state.player_y + rng.gen_range(-ZOMBIE_SPAWN_RADIUS..=ZOMBIE_SPAWN_RADIUS);

            if game_map.in_bounds(x, y) && game_map.tiles[game_map.xy_idx(x, y)] == TileType::Floor {
                // Check if already occupied by a zombie
                if !state.zombies.iter().any(|z| z.x == x && z.y == y) {
                    state.zombies.push(Zombie { x, y, hp: 10 }); // Initialize with HP
                    state.message_log.push("You hear distant groaning...".to_string());
                    break;
                }
            }
            attempts += 1;
        }
    }
}

// NEW FUNCTION: Tactical Retreat
fn handle_retreat_action(map: &Map, state: &mut GameState, rng: &mut ThreadRng) {
    state.fatigue = state.fatigue.saturating_sub(5); // Fatigue cost for the action
    state.hunger = state.hunger.saturating_sub(1);
    state.thirst = state.thirst.saturating_sub(1);

    // 1. Find the closest adjacent zombie (Manhattan distance = 1)
    if let Some(closest_zombie_index) = state.zombies.iter().position(|z| {
        (state.player_x - z.x).abs() + (state.player_y - z.y).abs() == 1
    }) {
        // 2. Check for item to throw
        // Requires more than just the primary weapon ("rusty knife" or "gun" counts as one item)
        if state.inventory.len() > 1 {
            // Find an item that is NOT the main weapon to throw
            let non_weapon_index = state.inventory.iter().rposition(|item| item != "rusty knife" && item != "gun");

            let thrown_item = if let Some(idx) = non_weapon_index {
                state.inventory.remove(idx)
            } else {
                // Fallback: throw the main weapon if nothing else is available (shouldn't happen due to len > 1 check, but safe)
                state.inventory.pop().unwrap_or("nothing".to_string())
            };
            
            state.message_log.push(format!("You threw your {} to distract the horde!", thrown_item));

            // 3. Remove the zombie
            let zombie = state.zombies.remove(closest_zombie_index);
            state.message_log.push(format!("Retreat successful! Zombie dispatched by distraction."));

            // 4. Calculate escape direction
            let dx = (state.player_x - zombie.x).signum();
            let dy = (state.player_y - zombie.y).signum();
            
            // Prioritize diagonal retreat if possible, otherwise move directly away
            let escape_targets = [(state.player_x + dx, state.player_y + dy), (state.player_x + dx, state.player_y), (state.player_x, state.player_y + dy)];
            
            let mut escaped = false;
            for (nx, ny) in escape_targets.iter() {
                // Check map bounds, walkability, and collision with other zombies
                if map.is_walkable(*nx, *ny) && !state.zombies.iter().any(|z| z.x == *nx && z.y == *ny) {
                    state.player_x = *nx;
                    state.player_y = *ny;
                    escaped = true;
                    state.message_log.push("You scrambled back to safety.".to_string());
                    break;
                }
            }

            if !escaped {
                 // Player couldn't move, but the distraction is still effective.
                 state.message_log.push("You couldn't move, but the distraction bought time.".to_string());
            }
            
        } else {
            state.message_log.push("You only have your main weapon! Cannot afford to retreat.".to_string());
        }
    } else {
        state.message_log.push("No adjacent zombie to retreat from.".to_string());
    }
    while state.message_log.len() > 10 { state.message_log.remove(0); }
}


// FIX: Corrected function signature to use &Map and &mut GameState
fn update_zombies(
    map: &Map,
    state: &mut GameState,
) {
    // FIX: Create an immutable HashSet of ALL CURRENT zombie locations
    let occupied_positions: HashSet<(i32, i32)> = state.zombies.iter().map(|z| (z.x, z.y)).collect();

    for zombie in state.zombies.iter_mut() {
        let zx = zombie.x;
        let zy = zombie.y;

        let dx = (state.player_x - zx).signum();
        let dy = (state.player_y - zy).signum();

        let dist = (state.player_x - zx).abs() + (state.player_y - zy).abs();

        if dist == 1 {
            // Already adjacent, attack and skip movement
            state.health = state.health.saturating_sub(1); // ADJUSTED DAMAGE: 1 HP
            state.message_log.push("A zombie bites you! (-1 HP)".to_string()); // ADJUSTED MESSAGE
            continue; 
        } else if dist > 1 && dist <= 12 { // Move only if in FOV radius (12)
            
            // Try positions in a fixed order: X-move then Y-move
            let try_positions = [(zx + dx, zy), (zx, zy + dy)];

            let mut moved = false;
            for &(nx, ny) in &try_positions {
                if !map.in_bounds(nx, ny) {
                    continue;
                }

                let target_tile = map.tiles[map.xy_idx(nx, ny)];
                let is_walkable_tile = target_tile != TileType::Wall;
                
                // Check if the target is the player OR if it's already occupied by a zombie.
                let is_occupied_by_another_zombie = occupied_positions.contains(&(nx, ny)) && (nx != zx || ny != zy); 
                
                if is_walkable_tile && !is_occupied_by_another_zombie {
                    // Update zombie position
                    zombie.x = nx;
                    zombie.y = ny;
                    moved = true;
                    break;
                }
            }

        }
    }
    
    while state.message_log.len() > 10 { state.message_log.remove(0); }
}

// --- Interaction Handlers ---

fn is_near_building(map: &Map, px: i32, py: i32) -> bool {
    for dy in -1..=1 {
        for dx in -1..=1 {
            if map.in_bounds(px + dx, py + dy) {
                let idx = map.xy_idx(px + dx, py + dy);
                if map.tiles[idx] == TileType::Building {
                    return true;
                }
            }
        }
    }
    false
}

fn rest_in_building(state: &mut GameState) {
    let hunger_cost = 10;
    let thirst_cost = 10;

    if state.hunger < hunger_cost || state.thirst < thirst_cost {
        state.message_log.push("Too hungry/thirsty to rest safely!".to_string());
        return;
    }

    state.fatigue = state.fatigue.saturating_add(50).min(100); 
    state.health = state.health.saturating_add(10).min(100);
    state.hunger = state.hunger.saturating_sub(hunger_cost); 
    state.thirst = state.thirst.saturating_sub(thirst_cost);

    state.message_log.push(format!("Rested well! Fatigue: +50, Health: +10. Hunger/Thirst: -{}.", hunger_cost).to_string());
    
    while state.message_log.len() > 10 { state.message_log.remove(0); }
}


// FIXED: Ensures combat is turn-ending without player movement, regardless of outcome.
fn handle_tile_interaction(map: &mut Map, state: &mut GameState, x: i32, y: i32, rng: &mut ThreadRng) {
    if !map.in_bounds(x, y) {
        state.message_log.push("Bump! You hit the edge of the world.".to_string());
        return;
    }

    // 1. Check for Zombie Combat (Turn-ending, no player movement)
    if state.zombies.iter().any(|z| z.x == x && z.y == y) {
        handle_attack(map, state, x, y, rng); 
        return; // Combat is turn-ending, player stays put.
    }

    let idx = map.xy_idx(x, y);
    let tile = map.tiles[idx];
    let mut moved = false;

    match tile {
        TileType::Wall => {
            state.message_log.push("Bump! A solid obstacle.".to_string());
        }
        TileType::Foliage => {
            state.fatigue = state.fatigue.saturating_sub(1);
            if rng.gen_bool(0.2) {
                let found = rng.gen_range(0..3);
                match found {
                    0 => { state.message_log.push("Scavenged! Found a moldy berry. (+5 Hunger)".to_string()); state.hunger = state.hunger.saturating_add(5).min(100); }
                    1 => { state.message_log.push("Scavenged! Found a damp leaf. (+5 Thirst)".to_string()); state.thirst = state.thirst.saturating_add(5).min(100); }
                    2 => { state.message_log.push("Found a bandage. (+5 Health)".to_string()); state.health = state.health.saturating_add(5).min(100); }
                    _ => {}
                }
                map.tiles[idx] = TileType::Floor;
            } else {
                state.message_log.push("Rustle, rustle... just leaves.".to_string());
            }
            moved = true; 
        }
        TileType::Car => {
            state.fatigue = state.fatigue.saturating_sub(3);
            if rng.gen_bool(0.7) { 
                let found = rng.gen_range(0..2);
                match found {
                    0 => { 
                        if !state.inventory.contains(&"lighter".to_string()) {
                            state.message_log.push("Car searched. Found a lighter and rags.".to_string()); 
                            state.inventory.push("lighter".to_string()); 
                        } else {
                            state.message_log.push("Car searched. Found some rags.".to_string());
                        }
                    }
                    1 => { state.message_log.push("Car searched. Found an energy bar! (+10 Hunger)".to_string()); state.hunger = state.hunger.saturating_add(10).min(100); }
                    _ => {}
                }
                map.tiles[idx] = TileType::Floor;
            } else {
                state.message_log.push("Car searched. Nothing useful but rust.".to_string());
            }
            moved = true; 
        }
        TileType::Resource => {
            state.message_log.push("Dedicated Resource Cache!".to_string());
            match rng.gen_range(0..3) {
                0 => { state.message_log.push("Found a water bottle. (+20 Thirst)".to_string()); state.thirst = state.thirst.saturating_add(20).min(100); }
                1 => { state.message_log.push("Found a can of beans. (+20 Hunger)".to_string()); state.hunger = state.hunger.saturating_add(20).min(100); }
                2 => { state.message_log.push("Found a first aid kit. (+15 Health)".to_string()); state.health = state.health.saturating_add(15).min(100); }
                _ => {}
            }
            map.tiles[idx] = TileType::Floor;
            moved = true;
        }
        TileType::Building => {
            state.message_log.push("Found a safe Building! Press 'r' to rest.".to_string());
            moved = true; 
        }
        TileType::Weapon => {
            if !state.inventory.contains(&"gun".to_string()) {
                state.message_log.push("Found a **GUN**! (+5 Ammo)".to_string());
                state.inventory.push("gun".to_string());
                state.ammo = state.ammo.saturating_add(5);
            } else {
                state.message_log.push("Found extra ammo.".to_string());
                state.ammo = state.ammo.saturating_add(5);
            }
            map.tiles[idx] = TileType::Floor;
            moved = true;
        }
        TileType::Mall => {
            state.message_log.push("You reached the Mall! The final challenge awaits...".to_string());
            moved = true;
        }
        TileType::Floor => {
            moved = true; 
        }
        _ => {
            state.message_log.push("Bump! Cannot move there.".to_string());
        }
    }

    if moved {
        state.player_x = x;
        state.player_y = y;
        state.fatigue = state.fatigue.saturating_sub(1);
    }
    
    while state.message_log.len() > 10 { state.message_log.remove(0); }
}

// --- Main function ---

fn main() -> Result<()> {
    color_eyre::install()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut rng = rand::thread_rng();
    let map_width = 80;
    let map_height = 50;
    let mut game_map = Map::new(map_width, map_height);
    game_map.generate_bsp(&mut rng);

    let mut state = GameState::new();
    
    // Player spawn logic
    if let Some(first_room) = game_map.rooms.first() {
        let (px, py) = first_room.center();
        let mut spawn_x = px;
        let mut spawn_y = py;
        
        for dy in -1..=1 {
            for dx in -1..=1 {
                if game_map.in_bounds(px + dx, py + dy) && game_map.tiles[game_map.xy_idx(px + dx, py + dy)] != TileType::Wall {
                    spawn_x = px + dx;
                    spawn_y = py + dy;
                    break;
                }
            }
        }
        
        if game_map.in_bounds(spawn_x, spawn_y) {
            state.player_x = spawn_x;
            state.player_y = spawn_y;
            let idx = game_map.xy_idx(spawn_x, spawn_y);
            // Ensure player spawn tile is Floor
            game_map.tiles[idx] = TileType::Floor; 
        } else {
            state.player_x = 1; 
            state.player_y = 1;
        }
    } else {
        state.player_x = 1; 
        state.player_y = 1;
    }

    // Spawn an initial zombie from the tunnel logic if it was set
    if game_map.rooms.len() > 1 {
        let (prev_x, prev_y) = game_map.rooms[0].center();
        let (curr_x, curr_y) = game_map.rooms[1].center();

        let (z_x, z_y) = if rng.gen_bool(0.5) {
            // H-tunnel position (if H-tunnel was built first)
            if prev_x != curr_x { ((prev_x + curr_x) / 2, prev_y) } else { (prev_x, (prev_y + curr_y) / 2) }
        } else {
            // V-tunnel position (if V-tunnel was built first)
            if prev_y != curr_y { (prev_x, (prev_y + curr_y) / 2) } else { ((prev_x + curr_x) / 2, curr_y) }
        };

        if game_map.in_bounds(z_x, z_y) {
            let idx = game_map.xy_idx(z_x, z_y);
            // Only add the zombie to the GameState if the tile type was set during map gen
            if game_map.tiles[idx] == TileType::Zombie {
                state.zombies.push(Zombie { x: z_x, y: z_y, hp: 10 }); // <-- INITIALIZE HP
            }
        }
    }

    loop {
        let mut turn_taken = false;
        
        // === INPUT HANDLING ===
        if event::poll(std::time::Duration::from_millis(150))? {
            if let Event::Key(key) = event::read()? {
                
                if key.code == KeyCode::Esc {
                    break;
                }

                // Check for the REST action
                if key.code == KeyCode::Char('r') {
                    if is_near_building(&game_map, state.player_x, state.player_y) {
                        rest_in_building(&mut state);
                        turn_taken = true;
                    } else {
                        state.message_log.push("You can only rest inside or near a Building ('B').".to_string());
                    }
                } 
                // NEW: Check for the RETREAT action
                else if key.code == KeyCode::Char('t') {
                    handle_retreat_action(&game_map, &mut state, &mut rng);
                    turn_taken = true;
                }

                // Movement/Interaction logic
                let (target_x, target_y) = match key.code {
                    KeyCode::Up | KeyCode::Char('w') => (state.player_x, state.player_y - 1),
                    KeyCode::Down | KeyCode::Char('s') => (state.player_x, state.player_y + 1),
                    KeyCode::Left | KeyCode::Char('a') => (state.player_x - 1, state.player_y),
                    KeyCode::Right | KeyCode::Char('d') => (state.player_x + 1, state.player_y),
                    _ => {
                        if !turn_taken { continue; } 
                        else { (state.player_x, state.player_y) }
                    }
                };

                if !turn_taken {
                    handle_tile_interaction(&mut game_map, &mut state, target_x, target_y, &mut rng);
                    turn_taken = true;
                }
            }
        }
        
        // If the player took any action (move, interact, rest, or retreat)
        if turn_taken {
            // === ZOMBIE TURN START ===
            spawn_random_zombies(&mut rng, &game_map, &mut state);
            update_zombies(&game_map, &mut state);
            // === ZOMBIE TURN END ===

            // Game turn logic (passive state decay)
            if state.hunger > 0 { state.hunger -= 1; }
            if state.thirst > 0 { state.thirst -= 1; }
            if state.fatigue > 0 { state.fatigue -= 1; }

            // Check for negative stats and apply damage/game over logic
            if state.hunger <= 0 || state.thirst <= 0 || state.fatigue <= 0 {
                 if state.hunger <= 0 { state.health = state.health.saturating_sub(1); }
                 if state.thirst <= 0 { state.health = state.health.saturating_sub(1); }
                 if state.fatigue <= 0 { state.health = state.health.saturating_sub(1); }

                 if state.health <= 0 {
                     state.message_log.push("You succumbed to the elements.".to_string());
                     break;
                 }
            }
        }


        let fov = game_map.compute_fov(state.player_x, state.player_y, 12);

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints([
                    Constraint::Percentage(70),
                    Constraint::Percentage(30),
                ].as_ref())
                .split(f.area());

            let map_area = chunks[0];
            let render_w = map_area.width as i32;
            let render_h = map_area.height as i32;

            let map_w_i32 = game_map.width as i32;
            let map_h_i32 = game_map.height as i32;
            
            let mut view_x_min = state.player_x.saturating_sub(render_w / 2);
            let mut view_y_min = state.player_y.saturating_sub(render_h / 2);
            
            if view_x_min + render_w >= map_w_i32 {
                 view_x_min = map_w_i32 - render_w;
            }
            if view_y_min + render_h >= map_h_i32 {
                 view_y_min = map_h_i32 - render_h;
            }
            
            view_x_min = view_x_min.max(0);
            view_y_min = view_y_min.max(0);
            
            let view_x_max = view_x_min + render_w;
            let view_y_max = view_y_min + render_h;
            
            // Collect zombie locations for efficient drawing
            let zombie_locations: HashMap<(i32, i32), i32> = state.zombies.iter().map(|z| ((z.x, z.y), z.hp)).collect();


            let mut map_lines = vec![];
            for world_y in view_y_min..view_y_max {
                let mut line = vec![];
                for world_x in view_x_min..view_x_max {

                    if !game_map.in_bounds(world_x, world_y) {
                        line.push(Span::styled(" ", Style::default().fg(Color::Black)));
                        continue;
                    }

                    let tile_pos = (world_x, world_y);
                    let idx = game_map.xy_idx(world_x, world_y);
                    let tile = game_map.tiles[idx];
                    let shade = fov.get(&tile_pos).copied().unwrap_or(Shade::Dark);

                    let c = if world_x == state.player_x && world_y == state.player_y {
                        Span::styled("@", Style::default().fg(Color::Yellow))
                    } else if zombie_locations.contains_key(&tile_pos) {
                        // Draw moving zombie (color based on shade)
                        let col = match shade {
                            Shade::Dark => Color::Rgb(139, 0, 0),
                            Shade::Dim => Color::Rgb(200, 0, 0),
                            Shade::Lit => Color::Red,
                            Shade::Bright => Color::LightRed,
                        };
                        Span::styled("Z", Style::default().fg(col))
                    } else {
                        // Draw static tile
                        let (ch, col) = match (tile, shade) {
                            (TileType::Wall, Shade::Dark) => ('▓', Color::DarkGray),
                            (TileType::Wall, Shade::Dim) => ('▒', Color::Gray),
                            (TileType::Wall, Shade::Lit) => ('#', Color::White),
                            (TileType::Wall, Shade::Bright) => ('#', Color::Cyan),
                            
                            (TileType::Floor, Shade::Dark) => (' ', Color::Black),
                            (TileType::Floor, Shade::Dim) => ('░', Color::DarkGray),
                            (TileType::Floor, Shade::Lit) => ('.', Color::White),
                            (TileType::Floor, Shade::Bright) => ('·', Color::Yellow),
                            
                            (TileType::Foliage, Shade::Dark) => (' ', Color::Black),
                            (TileType::Foliage, Shade::Dim) => ('~', Color::Green),
                            (TileType::Foliage, Shade::Lit) => ('"', Color::LightGreen),
                            (TileType::Foliage, Shade::Bright) => (',', Color::LightGreen),
                            
                            (TileType::Car, Shade::Dark) => (' ', Color::Black),
                            (TileType::Car, Shade::Dim) => ('C', Color::DarkGray),
                            (TileType::Car, Shade::Lit) => ('C', Color::Gray),
                            (TileType::Car, Shade::Bright) => ('C', Color::White),
                            
                            (TileType::Resource, Shade::Dark) => (' ', Color::Black),
                            (TileType::Resource, Shade::Dim) => ('$', Color::Yellow),
                            (TileType::Resource, Shade::Lit) => ('$', Color::LightYellow),
                            (TileType::Resource, Shade::Bright) => ('$', Color::LightYellow),
                            
                            (TileType::Building, Shade::Dark) => (' ', Color::Black),
                            (TileType::Building, Shade::Dim) => ('B', Color::Blue),
                            (TileType::Building, Shade::Lit) => ('B', Color::Blue),
                            (TileType::Building, Shade::Bright) => ('B', Color::LightBlue),
                            
                            (TileType::Mall, Shade::Dark) => ('?', Color::Magenta),
                            (TileType::Mall, Shade::Dim) => ('M', Color::LightMagenta),
                            (TileType::Mall, Shade::Lit) => ('M', Color::LightMagenta),
                            (TileType::Mall, Shade::Bright) => ('M', Color::LightMagenta),

                            (TileType::Weapon, Shade::Dark) => ('?', Color::Red),
                            (TileType::Weapon, Shade::Dim) => ('W', Color::Red),
                            (TileType::Weapon, Shade::Lit) => ('W', Color::Red),
                            (TileType::Weapon, Shade::Bright) => ('W', Color::LightRed),
                            
                            (TileType::Zombie, _) => ('Z', Color::Black), // Static zombie tile is now drawn as a placeholder
                        };
                        Span::styled(ch.to_string(), Style::default().fg(col))
                    };
                    line.push(c);
                }
                map_lines.push(Line::from(line));
            }

            let title = if game_map.rooms.is_empty() {
                Block::default().borders(Borders::ALL).title(Span::styled("Gen skimped? Rerun!", Color::Red))
            } else {
                Block::default().borders(Borders::ALL).title(format!("{} Rooms | WASD/Arrows | R: Rest | T: Retreat | ESC Quit | @ World({},{})", 
                    game_map.rooms.len(), state.player_x, state.player_y))
            };
            let map_widget = Paragraph::new(map_lines).block(title);
            f.render_widget(map_widget, chunks[0]);

            let hud_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints([
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                    Constraint::Percentage(40),
                ].as_ref())
                .split(chunks[1]);

            // Mini-map
            let mini_w = 10;
            let mini_h = 10;
            let mut mini_lines = vec![];
            for my in 0..mini_h {
                let mut mini_line = vec![];
                for mx in 0..mini_w {
                    let m_x = state.player_x + (mx as i32 - 5);
                    let m_y = state.player_y + (my as i32 - 5);
                    
                    let m_ch = if m_x == state.player_x && m_y == state.player_y {
                        Span::styled("@", Color::Yellow)
                    } else if zombie_locations.contains_key(&(m_x, m_y)) {
                        Span::styled("Z", Color::Red)
                    } else if game_map.in_bounds(m_x, m_y) {
                        let m_tile = game_map.tiles[game_map.xy_idx(m_x, m_y)];
                        // Use the static tile type here, but draw moving zombies on main map
                        match m_tile {
                            TileType::Wall => Span::styled("#", Color::Gray),
                            TileType::Floor | TileType::Zombie => Span::styled(".", Color::White), 
                            TileType::Foliage => Span::styled("~", Color::Green),
                            TileType::Car => Span::styled("C", Color::DarkGray),
                            TileType::Resource => Span::styled("$", Color::Yellow),
                            TileType::Building => Span::styled("B", Color::Blue),
                            TileType::Mall => Span::styled("M", Color::LightMagenta),
                            TileType::Weapon => Span::styled("W", Color::Red),
                        }
                    } else {
                        Span::styled(" ", Color::Black)
                    };
                    mini_line.push(m_ch);
                }
                mini_lines.push(Line::from(mini_line));
            }
            let mini_widget = Paragraph::new(mini_lines).block(Block::default().borders(Borders::ALL).title("Mini-Map (Local)"));
            f.render_widget(mini_widget, hud_chunks[0]);

            // Backpack
            let backpack_items: Vec<Row> = state.inventory.iter().map(|item| {
                Row::new(vec![Cell::from(Span::styled(item, Style::default().fg(Color::Green)))])
            }).collect();
            let backpack_table = Table::new(backpack_items, &[Constraint::Percentage(100)]).block(Block::default().borders(Borders::ALL).title(format!("Backpack ({} Ammo)", state.ammo)));
            f.render_widget(backpack_table, hud_chunks[1]);

            // Moodles
            let moodle_lines = vec![
                Line::from(vec![
                    Span::styled("HP:", Color::Red),
                    Span::styled(format!("{:3}", state.health.max(0)), Style::default().fg(if state.health < 25 { Color::Red } else if state.health < 75 { Color::Yellow } else { Color::Green })),
                ]),
                Line::from(vec![
                    Span::styled("Hunger:", Color::White),
                    Span::styled(format!("{:3}", state.hunger.max(0)), Style::default().fg(if state.hunger < 25 { Color::Red } else if state.hunger < 75 { Color::Yellow } else { Color::Green })),
                ]),
                Line::from(vec![
                    Span::styled("Thirst:", Color::White),
                    Span::styled(format!("{:3}", state.thirst.max(0)), Style::default().fg(if state.thirst < 25 { Color::Red } else if state.thirst < 75 { Color::Yellow } else { Color::Green })),
                ]),
                Line::from(vec![
                    Span::styled("Fatigue:", Color::White),
                    Span::styled(format!("{:3}", state.fatigue.max(0)), Style::default().fg(if state.fatigue < 25 { Color::Red } else if state.fatigue < 75 { Color::Yellow } else { Color::Green })),
                ]),
            ];
            let moodle_widget = Paragraph::new(moodle_lines).block(Block::default().borders(Borders::ALL).title("Moodles"));
            f.render_widget(moodle_widget, hud_chunks[2]);

            // Dialogues
            let max_lines = hud_chunks[3].height as usize - 2; 
            let msg_lines: Vec<Line<'_>> = state.message_log
                .iter()
                .rev()
                .take(max_lines)
                .map(|msg| Line::from(Span::styled(msg.as_str(), Color::Cyan)))
                .collect();
            let mut final_lines = msg_lines.into_iter().rev().collect::<Vec<_>>();

            while final_lines.len() < max_lines {
                final_lines.insert(0, Line::from(""));
            }

            let dialogue_widget = Paragraph::new(final_lines).block(Block::default().borders(Borders::ALL).title("Radio/Log"));
            f.render_widget(dialogue_widget, hud_chunks[3]);
        })?;

        // Check death one final time before breaking
        if state.health <= 0 {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
    )?;
    terminal.show_cursor()?;

    println!("\nGame Over. {} rooms explored. Final pos: ({},{})", game_map.rooms.len(), state.player_x, state.player_y);
    Ok(())
}