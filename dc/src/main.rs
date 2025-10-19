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

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TileType {
    Wall,
    Floor,
    Zombie,
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
        if rng.gen_bool(0.1) {
            let (cx, cy) = room.center();
            let z_idx = self.xy_idx(cx, cy + 1);
            if self.in_bounds(cx, cy + 1) {
                self.tiles[z_idx] = TileType::Zombie;
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
            self.apply_h_tunnel(prev_x, curr_x, prev_y);
            self.apply_v_tunnel(curr_y, prev_y, curr_x);
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
                let mut blocked = false;
                if self.tiles[self.xy_idx(tx, ty)] == TileType::Wall {
                    blocked = true;
                }

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

struct GameState {
    player_x: i32,
    player_y: i32,
    hunger: i32,
    fatigue: i32,
    inventory: Vec<String>,
    message_log: Vec<String>,
}

impl GameState {
    fn new() -> Self {
        Self {
            player_x: 0,
            player_y: 0,
            hunger: 100,
            fatigue: 100,
            inventory: vec!["rusty knife".to_string()],
            message_log: vec!["Radio crackles: 'Help, the horde's at the mall!'".to_string()],
        }
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
    println!("Zomboid clone: Awakening in the apocalypse—hold tight!");

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut rng = rand::thread_rng();
    let mut game_map = Map::new(80, 50);
    game_map.generate_bsp(&mut rng);

    let mut state = GameState::new();
    if let Some(first_room) = game_map.rooms.first() {
        let (px, py) = first_room.center();
        if game_map.in_bounds(px, py) {
            state.player_x = px;
            state.player_y = py;
            let idx = game_map.xy_idx(px, py);
            game_map.tiles[idx] = TileType::Floor;
        } else {
            state.player_x = 1; // Fallback spawn
            state.player_y = 1;
        }
    } else {
        state.player_x = 1; // Fallback if no rooms
        state.player_y = 1;
    }

    loop {
        state.hunger -= 1;
        if state.hunger < 50 {
            state.message_log.push("Hunger gnaws—scavenge soon!".to_string());
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
            let render_w = map_area.width.max(1) as usize;
            let render_h = map_area.height.max(1) as usize;

            let cam_radius = 10i32;
            let view_x_min = (state.player_x.saturating_sub(cam_radius)).max(0);
            let view_y_min = (state.player_y.saturating_sub(cam_radius)).max(0);
            let view_x_max = (view_x_min + render_w as i32).min(game_map.width as i32 - 1);
            let view_y_max = (view_y_min + render_h as i32).min(game_map.height as i32 - 1);

            let mut map_lines = vec![];
            for dy in 0..render_h as i32 {
                let world_y = view_y_min + dy;
                let mut line = vec![];
                for dx in 0..render_w as i32 {
                    let world_x = view_x_min + dx;
                    let dist = (world_x - state.player_x).abs() as f32 + (world_y - state.player_y).abs() as f32;
                    let scale = (1.0 / (1.0 + dist * 0.05)).clamp(0.5, 1.0);

                    let idx = game_map.xy_idx(world_x, world_y);
                    let tile = if idx < game_map.tiles.len() { game_map.tiles[idx] } else { TileType::Wall };
                    let shade = fov.get(&(world_x, world_y)).copied().unwrap_or(Shade::Dark);

                    let (ch, col) = match (tile, shade) {
                        (TileType::Wall, Shade::Dark) => if scale > 0.8 { ('?', Color::DarkGray) } else { ('▓', Color::Black) },
                        (TileType::Wall, Shade::Dim) => if scale > 0.8 { ('#', Color::Gray) } else { ('▒', Color::DarkGray) },
                        (TileType::Wall, Shade::Lit) => if scale > 0.8 { ('#', Color::White) } else { ('▓', Color::White) },
                        (TileType::Wall, Shade::Bright) => if scale > 0.8 { ('#', Color::Cyan) } else { ('▓', Color::Cyan) },
                        (TileType::Zombie, Shade::Dark) => ('?', Color::Rgb(139, 0, 0)),
                        (TileType::Zombie, Shade::Dim) => ('g', Color::Green),
                        (TileType::Zombie, Shade::Lit) => ('g', Color::Yellow),
                        (TileType::Zombie, Shade::Bright) => ('G', Color::Red),
                        (TileType::Floor, Shade::Dark) => (' ', Color::Black),
                        (TileType::Floor, Shade::Dim) => if scale > 0.8 { ('.', Color::DarkGray) } else { ('░', Color::DarkGray) },
                        (TileType::Floor, Shade::Lit) => if scale > 0.8 { (':', Color::White) } else { ('.', Color::White) },
                        (TileType::Floor, Shade::Bright) => if scale > 0.8 { ('·', Color::Yellow) } else { ('.', Color::Yellow) },
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
                Block::default().borders(Borders::ALL).title(format!("{} Rooms | WASD/Arrows | ESC Quit | @ World({},{}) View({},{})", 
                    game_map.rooms.len(), state.player_x, state.player_y, view_x_min, view_y_min))
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
                    let m_x = state.player_x as i32 + (mx as i32 - 5);
                    let m_y = state.player_y as i32 + (my as i32 - 5);
                    let m_tile = if game_map.in_bounds(m_x, m_y) { game_map.tiles[game_map.xy_idx(m_x, m_y)] } else { TileType::Wall };
                    let m_ch = match m_tile {
                        TileType::Wall => Span::styled("#", Style::default().fg(Color::Gray)),
                        TileType::Floor => Span::styled(".", Style::default().fg(Color::White)),
                        TileType::Zombie => Span::styled("g", Style::default().fg(Color::Red)),
                    };
                    mini_line.push(m_ch);
                }
                mini_lines.push(Line::from(mini_line));
            }
            let mini_widget = Paragraph::new(mini_lines).block(Block::default().borders(Borders::ALL).title("Mini-Map"));
            f.render_widget(mini_widget, hud_chunks[0]);

            // Backpack
            // NEW (Corrected)
            let backpack_items: Vec<Row> = state.inventory.iter().map(|item| {
                let owned_string = item.clone(); // Clone the String here
                Row::new(vec![Cell::from(Span::styled(owned_string, Style::default().fg(Color::Green)))])
            }).collect();
            let backpack_table = Table::new(backpack_items, &[Constraint::Percentage(100)]).block(Block::default().borders(Borders::ALL).title("Backpack"));
            f.render_widget(backpack_table, hud_chunks[1]);

            // Moodles
            let moodle_lines = vec![Line::from(vec![
                Span::styled("Hunger:", Style::default().fg(Color::White)),
                Span::styled(format!("{:3}", state.hunger), Style::default().fg(if state.hunger < 50 { Color::Red } else { Color::Green })),
                Span::styled(" | Fatigue:", Style::default().fg(Color::White)),
                Span::styled(format!("{:3}", state.fatigue), Style::default().fg(if state.fatigue < 50 { Color::Red } else { Color::Green })),
            ])];
            let moodle_widget = Paragraph::new(moodle_lines).block(Block::default().borders(Borders::ALL).title("Moodles"));
            f.render_widget(moodle_widget, hud_chunks[2]);

            // Dialogues
            let msg_lines: Vec<Line<'_>> = state.message_log.iter().rev().take(8).map(|msg| Line::from(Span::styled(msg.as_str(), Style::default().fg(Color::Cyan)))).collect();
            let dialogue_widget = Paragraph::new(msg_lines).block(Block::default().borders(Borders::ALL).title("Radio/Log"));
            f.render_widget(dialogue_widget, hud_chunks[3]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => break,
                KeyCode::Up | KeyCode::Char('w') => {
                    let (nx, ny) = try_move_player(&game_map, state.player_x, state.player_y, 0, -1);
                    if nx != state.player_x || ny != state.player_y {
                        state.player_x = nx;
                        state.player_y = ny;
                        state.message_log.push("Moved north—horde groans in distance...".to_string());
                    } else {
                        state.message_log.push("Bump! Barricade or back off.".to_string());
                    }
                    if state.message_log.len() > 10 { state.message_log.remove(0); }
                }
                KeyCode::Down | KeyCode::Char('s') => {
                    let (nx, ny) = try_move_player(&game_map, state.player_x, state.player_y, 0, 1);
                    if nx != state.player_x || ny != state.player_y {
                        state.player_x = nx;
                        state.player_y = ny;
                        state.message_log.push("Moved south—the floor creaks ominously...".to_string());
                    } else {
                        state.message_log.push("Bump! Barricade or back off.".to_string());
                    }
                    if state.message_log.len() > 10 { state.message_log.remove(0); }
                }
                KeyCode::Left | KeyCode::Char('a') => {
                    let (nx, ny) = try_move_player(&game_map, state.player_x, state.player_y, -1, 0);
                    if nx != state.player_x || ny != state.player_y {
                        state.player_x = nx;
                        state.player_y = ny;
                        state.message_log.push("Moved west—something skitters in the shadows...".to_string());
                    } else {
                        state.message_log.push("Bump! Barricade or back off.".to_string());
                    }
                    if state.message_log.len() > 10 { state.message_log.remove(0); }
                }
                KeyCode::Right | KeyCode::Char('d') => {
                    let (nx, ny) = try_move_player(&game_map, state.player_x, state.player_y, 1, 0);
                    if nx != state.player_x || ny != state.player_y {
                        state.player_x = nx;
                        state.player_y = ny;
                        state.message_log.push("Moved east—a faint glow ahead...".to_string());
                    } else {
                        state.message_log.push("Bump! Barricade or back off.".to_string());
                    }
                    if state.message_log.len() > 10 { state.message_log.remove(0); }
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

    println!("\nDungeon dusted—{} rooms explored. GG! Final pos: ({},{})", game_map.rooms.len(), state.player_x, state.player_y);
    Ok(())
}