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

// --- Enums and Structs (Unchanged) ---

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
    Dark,    // ? for unknown
    Dim,     // . faint
    Lit,     // : lit
    Bright,  // · bright/full
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
        self.in_bounds(x, y) && self.tiles[self.xy_idx(x, y)] != TileType::Wall && self.tiles[self.xy_idx(x, y)] != TileType::Zombie
    }

    pub fn apply_room(&mut self, room: &Rect, rng: &mut ThreadRng) {
        for y in room.y1..=room.y2 {
            for x in room.x1..=room.x2 {
                if self.in_bounds(x, y) {
                    let idx = self.xy_idx(x, y);
                    self.tiles[idx] = TileType::Floor;
                }
            }
        }
        // Zombie spawn tease 
        if rng.gen_bool(0.15) {
            let (cx, cy) = room.center();
            if self.in_bounds(cx, cy + 1) {
                 let z_idx = self.xy_idx(cx, cy + 1);
                 if self.tiles[z_idx] == TileType::Floor {
                    self.tiles[z_idx] = TileType::Zombie;
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
        println!("BSP gen firing—target 10-20 rooms");
        let mut rects = vec![Rect::new(1, 1, self.width as i32 - 2, self.height as i32 - 2)];

        while !rects.is_empty() && self.rooms.len() < 20 {
            let idx = rng.gen_range(0..rects.len());
            let current = rects.swap_remove(idx);

            let room = current.create_room(rng);
            let valid = !self.rooms.iter().any(|r| room.intersects(r));
            if valid {
                self.rooms.push(room);
                self.apply_room(&room, rng);
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
        }

        // Apply random features (Foliage, Cars, Resources, Buildings, Weapons) to all rooms
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                let idx = self.xy_idx(x, y);
                if self.tiles[idx] == TileType::Floor && rng.gen_bool(0.08) { 
                    match rng.gen_range(0..15) { 
                        0..=8 => self.tiles[idx] = TileType::Foliage,
                        9..=11 => self.tiles[idx] = TileType::Car,
                        12 => self.tiles[idx] = TileType::Resource,
                        13 => self.tiles[idx] = TileType::Building,
                        14 => self.tiles[idx] = TileType::Weapon,
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
        
        // Final sanity check for Room 0: Ensure the starting room has no Mall tiles.
        if let Some(first_room) = self.rooms.first() {
            for y in first_room.y1..=first_room.y2 {
                for x in first_room.x1..=first_room.x2 {
                    if self.in_bounds(x, y) {
                        let idx = self.xy_idx(x, y);
                        if self.tiles[idx] == TileType::Mall {
                            self.tiles[idx] = TileType::Floor;
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

// --- Game State ---

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
        }
    }
}

// --- Interaction Handlers ---

/// Checks if the player is on or adjacent to a TileType::Building
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

/// Allows the player to rest and recover stats.
fn rest_in_building(state: &mut GameState) {
    // Large Fatigue Recovery
    state.fatigue = state.fatigue.saturating_add(50).min(100); 
    // Small Health Recovery
    state.health = state.health.saturating_add(10).min(100);
    // Cost of time spent resting
    state.hunger = state.hunger.saturating_sub(10); 
    state.thirst = state.thirst.saturating_sub(10);

    state.message_log.push(format!("Rested well! Fatigue: +50, Health: +10. Hunger/Thirst: -10.").to_string());
}


fn handle_tile_interaction(map: &mut Map, state: &mut GameState, x: i32, y: i32, rng: &mut ThreadRng) {
    if !map.in_bounds(x, y) {
        state.message_log.push("Bump! You hit the edge of the world.".to_string());
        return;
    }

    let idx = map.xy_idx(x, y);
    let tile = map.tiles[idx];
    let mut moved = false;

    match tile {
        TileType::Wall => {
            state.message_log.push("Bump! A solid obstacle.".to_string());
        }
        TileType::Zombie => {
            state.fatigue = state.fatigue.saturating_sub(5);
            
            // Combat chance calculation now includes Fatigue (Higher Fatigue = Lower Combat Chance)
            // Base chance (0.5) + Fatigue bonus (0 to 0.4) = Max 0.9
            let fatigue_modifier = (state.fatigue.max(0) as f32 / 100.0) * 0.4;
            let base_chance = 0.5;
            let hit_chance = if state.inventory.contains(&"gun".to_string()) && state.ammo > 0 { 
                0.9 
            } else { 
                base_chance + fatigue_modifier
            };
            
            if rng.gen_bool(hit_chance as f64) {
                state.message_log.push(format!("Zombie hit! Chance: {:.2}", hit_chance).to_string());
                if state.inventory.contains(&"gun".to_string()) && state.ammo > 0 {
                    state.ammo -= 1;
                    state.message_log.push(format!("*BANG* Zombie dispatched! (-1 Ammo)").to_string());
                    map.tiles[idx] = TileType::Floor;
                } else if rng.gen_bool(0.7) {
                    state.message_log.push("Rusty knife dispatched the zombie.".to_string());
                    map.tiles[idx] = TileType::Floor;
                } else {
                    state.message_log.push("Rusty knife glance off its tough hide.".to_string());
                }
            } else {
                state.message_log.push(format!("Zombie retaliates! (-5 Health, -5 Fatigue). Chance: {:.2}", hit_chance).to_string());
                state.health = state.health.saturating_sub(5);
                state.fatigue = state.fatigue.saturating_sub(5);
            }
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
            if rng.gen_bool(0.4) {
                state.message_log.push("Car searched. Found scrap metal.".to_string());
                state.inventory.push("scrap metal".to_string());
                map.tiles[idx] = TileType::Floor;
            } else {
                state.message_log.push("Car searched. Nothing useful.".to_string());
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
            moved = true; // Player moves onto the building tile
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
    }

    if moved {
        state.player_x = x;
        state.player_y = y;
        state.fatigue = state.fatigue.saturating_sub(1);
    }
    
    while state.message_log.len() > 10 { state.message_log.remove(0); }
}

// --- Main function with Resting Keybind ---

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
            game_map.tiles[idx] = TileType::Floor; 
        } else {
            state.player_x = 1; 
            state.player_y = 1;
        }
    } else {
        state.player_x = 1; 
        state.player_y = 1;
    }

    loop {
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


            let mut map_lines = vec![];
            for world_y in view_y_min..view_y_max {
                let mut line = vec![];
                for world_x in view_x_min..view_x_max {

                    if !game_map.in_bounds(world_x, world_y) {
                        line.push(Span::styled(" ", Style::default().fg(Color::Black)));
                        continue;
                    }

                    let _dist = ((world_x - state.player_x).abs() as f32 + (world_y - state.player_y).abs() as f32);

                    let idx = game_map.xy_idx(world_x, world_y);
                    let tile = game_map.tiles[idx];
                    let shade = fov.get(&(world_x, world_y)).copied().unwrap_or(Shade::Dark);

                    let (ch, col) = match (tile, shade) {
                        (TileType::Wall, Shade::Dark) => ('▓', Color::DarkGray),
                        (TileType::Wall, Shade::Dim) => ('▒', Color::Gray),
                        (TileType::Wall, Shade::Lit) => ('#', Color::White),
                        (TileType::Wall, Shade::Bright) => ('#', Color::Cyan),
                        
                        (TileType::Zombie, Shade::Dark) => ('?', Color::Rgb(139, 0, 0)),
                        (TileType::Zombie, Shade::Dim) => ('g', Color::Green),
                        (TileType::Zombie, Shade::Lit) => ('g', Color::Yellow),
                        (TileType::Zombie, Shade::Bright) => ('G', Color::Red),
                        
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
                    };

                    let c = if world_x == state.player_x && world_y == state.player_y {
                        Span::styled("@", Style::default().fg(Color::Yellow))
                    } else {
                        Span::styled(ch.to_string(), Style::default().fg(col))
                    };
                    line.push(c);
                }
                map_lines.push(Line::from(line));
            }

            let title = if game_map.rooms.is_empty() {
                Block::default().borders(Borders::ALL).title(Span::styled("Gen skimped? Rerun!", Style::default().fg(Color::Red)))
            } else {
                Block::default().borders(Borders::ALL).title(format!("{} Rooms | WASD/Arrows | R: Rest | ESC Quit | @ World({},{})", 
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
                        Span::styled("@", Style::default().fg(Color::Yellow))
                    } else if game_map.in_bounds(m_x, m_y) {
                        let m_tile = game_map.tiles[game_map.xy_idx(m_x, m_y)];
                        match m_tile {
                            TileType::Wall => Span::styled("#", Style::default().fg(Color::Gray)),
                            TileType::Floor => Span::styled(".", Style::default().fg(Color::White)),
                            TileType::Zombie => Span::styled("g", Style::default().fg(Color::Red)),
                            TileType::Foliage => Span::styled("~", Style::default().fg(Color::Green)),
                            TileType::Car => Span::styled("C", Style::default().fg(Color::DarkGray)),
                            TileType::Resource => Span::styled("$", Style::default().fg(Color::Yellow)),
                            TileType::Building => Span::styled("B", Style::default().fg(Color::Blue)),
                            TileType::Mall => Span::styled("M", Style::default().fg(Color::LightMagenta)),
                            TileType::Weapon => Span::styled("W", Style::default().fg(Color::Red)),
                        }
                    } else {
                        Span::styled(" ", Style::default().fg(Color::Black))
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
                    Span::styled("HP:", Style::default().fg(Color::Red)),
                    Span::styled(format!("{:3}", state.health.max(0)), Style::default().fg(if state.health < 25 { Color::Red } else if state.health < 75 { Color::Yellow } else { Color::Green })),
                ]),
                Line::from(vec![
                    Span::styled("Hunger:", Style::default().fg(Color::White)),
                    Span::styled(format!("{:3}", state.hunger.max(0)), Style::default().fg(if state.hunger < 25 { Color::Red } else if state.hunger < 75 { Color::Yellow } else { Color::Green })),
                ]),
                Line::from(vec![
                    Span::styled("Thirst:", Style::default().fg(Color::White)),
                    Span::styled(format!("{:3}", state.thirst.max(0)), Style::default().fg(if state.thirst < 25 { Color::Red } else if state.thirst < 75 { Color::Yellow } else { Color::Green })),
                ]),
                Line::from(vec![
                    Span::styled("Fatigue:", Style::default().fg(Color::White)),
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
                .map(|msg| Line::from(Span::styled(msg.as_str(), Style::default().fg(Color::Cyan))))
                .collect();
            let mut final_lines = msg_lines.into_iter().rev().collect::<Vec<_>>();

            while final_lines.len() < max_lines {
                final_lines.insert(0, Line::from(""));
            }

            let dialogue_widget = Paragraph::new(final_lines).block(Block::default().borders(Borders::ALL).title("Radio/Log"));
            f.render_widget(dialogue_widget, hud_chunks[3]);
        })?;

        // Event handling - Added 'r' for Rest
        if let Event::Key(key) = event::read()? {
            
            if key.code == KeyCode::Esc {
                break;
            }

            // Check for the REST action first
            if key.code == KeyCode::Char('r') {
                if is_near_building(&game_map, state.player_x, state.player_y) {
                    rest_in_building(&mut state);
                } else {
                    state.message_log.push("You can only rest inside or near a Building ('B').".to_string());
                }
                while state.message_log.len() > 10 { state.message_log.remove(0); }
                continue; // Skip movement logic for this turn
            }

            // Movement logic
            let (target_x, target_y) = match key.code {
                KeyCode::Up | KeyCode::Char('w') => (state.player_x, state.player_y - 1),
                KeyCode::Down | KeyCode::Char('s') => (state.player_x, state.player_y + 1),
                KeyCode::Left | KeyCode::Char('a') => (state.player_x - 1, state.player_y),
                KeyCode::Right | KeyCode::Char('d') => (state.player_x + 1, state.player_y),
                _ => continue,
            };

            // Call the handler
            handle_tile_interaction(&mut game_map, &mut state, target_x, target_y, &mut rng);
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