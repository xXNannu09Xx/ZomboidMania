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
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;
use std::collections::HashMap;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TileType {
    Wall,
    Floor,
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
        x >= 1 && x < self.width as i32 - 1 && y >= 1 && y < self.height as i32 - 1
    }

    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        self.in_bounds(x, y) && self.tiles[self.xy_idx(x, y)] == TileType::Floor
    }

    pub fn apply_room(&mut self, room: &Rect) {
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
            let idx = self.xy_idx(x, y);
            if self.in_bounds(x, y) && self.tiles[idx] == TileType::Wall {
                self.tiles[idx] = TileType::Floor;
            }
        }
    }

    pub fn apply_v_tunnel(&mut self, y1: i32, y2: i32, x: i32) {
        let min_y = std::cmp::min(y1, y2);
        let max_y = std::cmp::max(y1, y2);
        for y in min_y..=max_y {
            let idx = self.xy_idx(x, y);
            if self.in_bounds(x, y) && self.tiles[idx] == TileType::Wall {
                self.tiles[idx] = TileType::Floor;
            }
        }
    }

    pub fn generate_bsp(&mut self, rng: &mut ThreadRng) {
        println!("BSP gen firing—target 10-20 rooms");
        let mut rects = vec![Rect::new(2, 2, self.width as i32 - 3, self.height as i32 - 3)];

        while !rects.is_empty() && self.rooms.len() < 20 {
            let idx = rng.gen_range(0..rects.len());
            let current = rects.swap_remove(idx);

            let room = current.create_room(rng);
            let valid = !self.rooms.iter().any(|r| room.intersects(r));
            if valid {
                self.rooms.push(room);
                self.apply_room(&room);
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
            self.apply_h_tunnel(prev_x, curr_x, prev_y);
            self.apply_v_tunnel(curr_y, prev_y, curr_x);
        }
    }

    // FOV: Simple raycast from player, blocks on walls, shades by dist
    pub fn compute_fov(&self, px: i32, py: i32, radius: i32) -> HashMap<(i32, i32), Shade> {
        let mut visible: HashMap<(i32, i32), Shade> = HashMap::new();
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy > radius * radius { continue; }
                let tx = px + dx;
                let ty = py + dy;
                if !self.in_bounds(tx, ty) { continue; }

                // Basic LOS: Check if path blocked (dummy for now; add Bresenham rays later)
                let dist = ((dx * dx + dy * dy) as f32).sqrt();  // Euclidean dist
                let mut blocked = false;
                if self.tiles[self.xy_idx(tx, ty)] == TileType::Wall {
                    blocked = true;
                }  // Expand: Step along line, break on wall

                let shade = if blocked {
                    Shade::Dark
                } else if dist > (radius as f32) * 0.7f32 {  // Fix: Cast radius to f32
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

fn try_move_player(map: &Map, x: i32, y: i32, dx: i32, dy: i32) -> (i32, i32) {
    let new_x = x + dx;
    let new_y = y + dy;
    if map.is_walkable(new_x, new_y) {
        (new_x, new_y)
    } else {
        (x, y)
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    println!("Roguelike awakening—hold tight!");

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut rng = rand::thread_rng();
    let mut game_map = Map::new(80, 50);
    game_map.generate_bsp(&mut rng);

    let mut player_x = 0i32;
    let mut player_y = 0i32;
    let mut message_log: Vec<String> = vec!["Welcome to the dungeon. Explore with WASD/Arrows.".to_string()];
    if let Some(first_room) = game_map.rooms.first() {
        let (px, py) = first_room.center();
        player_x = px;
        player_y = py;
        if game_map.in_bounds(px, py) {
            let idx = game_map.xy_idx(px, py);
            game_map.tiles[idx] = TileType::Floor;
        }
    }

    loop {
        let fov = game_map.compute_fov(player_x, player_y, 12);  // Radius for depth tease

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([
                    Constraint::Min(20),
                    Constraint::Max(5),
                ].as_ref())
                .split(f.area());

            let map_area = chunks[0];
            let render_w = map_area.width.max(1) as usize;
            let render_h = map_area.height.max(1) as usize;

            let cam_radius = 10i32;
            let view_x_min = (player_x.saturating_sub(cam_radius)).max(0);
            let view_y_min = (player_y.saturating_sub(cam_radius)).max(0);
            let view_x_max = (view_x_min + render_w as i32).min(game_map.width as i32 - 1);
            let view_y_max = (view_y_min + render_h as i32).min(game_map.height as i32 - 1);

            let mut map_lines = vec![];
            for dy in 0..render_h as i32 {
                let world_y = view_y_min + dy;  // Fix: Declare world_y here
                let mut line = vec![];
                for dx in 0..render_w as i32 {
                    let world_x = view_x_min + dx;  // Fix: Declare world_x here
                    let idx = game_map.xy_idx(world_x, world_y);
                    let tile = if idx < game_map.tiles.len() { game_map.tiles[idx] } else { TileType::Wall };  // Fix: Declare tile here
                    let shade = fov.get(&(world_x, world_y)).copied().unwrap_or(Shade::Dark);  // Fix: Use get, == to && logic

                    let (ch, col) = match (tile, shade) {
                        (TileType::Wall, Shade::Dark) => ('?', Color::DarkGray),
                        (TileType::Wall, Shade::Dim) => ('#', Color::Gray),
                        (TileType::Wall, Shade::Lit) => ('#', Color::White),
                        (TileType::Wall, Shade::Bright) => ('#', Color::Cyan),
                        (TileType::Floor, Shade::Dark) => (' ', Color::Black),
                        (TileType::Floor, Shade::Dim) => ('.', Color::DarkGray),
                        (TileType::Floor, Shade::Lit) => (':', Color::White),
                        (TileType::Floor, Shade::Bright) => ('·', Color::Yellow),
                    };
                    let c = if world_x == player_x && world_y == player_y {
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
                Block::default().borders(Borders::ALL).title(format!("{} Rooms | WASD/Arrows | ESC Quit | @ World({},{}) View({},{})", 
                    game_map.rooms.len(), player_x, player_y, view_x_min, view_y_min))
            };
            let map_widget = Paragraph::new(map_lines).block(title);
            f.render_widget(map_widget, chunks[0]);

            let msg = message_log.last().cloned().unwrap_or_else(|| "No echoes yet...".to_string());
            let mut msg_lines = vec![Line::from(Span::styled(msg.as_str(), 
                Style::default().fg(Color::Green)))];
            let msg_block = Block::default().borders(Borders::ALL).title("Echoes");
            let msg_widget = Paragraph::new(msg_lines).block(msg_block);
            f.render_widget(msg_widget, chunks[1]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => break,
                KeyCode::Up | KeyCode::Char('w') => {
                    let (nx, ny) = try_move_player(&game_map, player_x, player_y, 0, -1);
                    if nx != player_x || ny != player_y {
                        player_x = nx;
                        player_y = ny;
                        message_log.push("Moved north—echoes in the dark...".to_string());
                    } else {
                        message_log.push("Bump! Solid wall blocks your path.".to_string());
                    }
                    if message_log.len() > 3 { message_log.remove(0); }
                }
                KeyCode::Down | KeyCode::Char('s') => {
                    let (nx, ny) = try_move_player(&game_map, player_x, player_y, 0, 1);
                    if nx != player_x || ny != player_y {
                        player_x = nx;
                        player_y = ny;
                        message_log.push("Moved south—the floor creaks ominously...".to_string());
                    } else {
                        message_log.push("Bump! Solid wall blocks your path.".to_string());
                    }
                    if message_log.len() > 3 { message_log.remove(0); }
                }
                KeyCode::Left | KeyCode::Char('a') => {
                    let (nx, ny) = try_move_player(&game_map, player_x, player_y, -1, 0);
                    if nx != player_x || ny != player_y {
                        player_x = nx;
                        player_y = ny;
                        message_log.push("Moved west—something skitters in the shadows...".to_string());
                    } else {
                        message_log.push("Bump! Solid wall blocks your path.".to_string());
                    }
                    if message_log.len() > 3 { message_log.remove(0); }
                }
                KeyCode::Right | KeyCode::Char('d') => {
                    let (nx, ny) = try_move_player(&game_map, player_x, player_y, 1, 0);
                    if nx != player_x || ny != player_y {
                        player_x = nx;
                        player_y = ny;
                        message_log.push("Moved east—a faint glow ahead...".to_string());
                    } else {
                        message_log.push("Bump! Solid wall blocks your path.".to_string());
                    }
                    if message_log.len() > 3 { message_log.remove(0); }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
    )?;
    terminal.show_cursor()?;

    println!("\nDungeon dusted—{} rooms explored. GG! Final pos: ({},{})", game_map.rooms.len(), player_x, player_y);
    Ok(())
}