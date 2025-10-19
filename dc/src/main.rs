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
    widgets::{Block, Borders, Paragraph, Table, TableState},
    Terminal,
};
use std::io;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TileType {
    Wall,
    Floor,
    Zombie,  // New: g for shambler
}

#[derive(PartialEq, Eq, clone, Copy, Debug)]
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

// ... (Rect impl same as before)

#[derive(Clone, Debug)]
pub struct Map {
    pub tiles: Vec<TileType>,
    pub width: usize,
    pub height: usize,
    pub rooms: Vec<Rect>,
}

// ... (Map impl same as before, but add TileType::Zombie to tiles in gen if rng rolls)

struct GameState {
    player_x: i32,
    player_y: i32,
    hunger: i32,  // 0-100, ticks down
    fatigue: i32, // 0-100
    inventory: Vec<String>,  // Backpack: "canned beans", "baseball bat"
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
    println!("Zomboid build: Awakening in the apocalypse—hold tight!");

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut rng = rand::thread_rng();
    let mut game_map = Map::new(80, 50);
    game_map.generate_bsp(&mut rng);

    let mut state = GameState::new();
    // Spawn player in first room
    if let Some(first_room) = game_map.rooms.first() {
        let (px, py) = first_room.center();
        state.player_x = px;
        state.player_y = py;
        let idx = game_map.xy_idx(px, py);
        game_map.tiles[idx] = TileType::Floor;
    }

    loop {
        state.hunger -= 1;  // Tick down for risk
        if state.hunger < 50 { state.message_log.push("Hunger gnaws—scavenge soon!".to_string()); }

        let fov = game_map.compute_fov(state.player_x, state.player_y, 12);

        terminal.draw(|f| {
            // Main split: Horizontal for left/right
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints([
                    Constraint::Percentage(70),  // Left: Map
                    Constraint::Percentage(30),  // Right: HUD stack
                ].as_ref())
                .split(f.area());

            // LEFT: Map grid (horizontal rectangle)
            let map_area = chunks[0];
            let render_w = map_area.width.max(1) as usize;
            let render_h = map_area.height.max(1) as usize;

            let cam_radius = 15i32;
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
                        (TileType::Zombie, Shade::Dark) => ('?', Color::Red),
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

            let map_title = if game_map.rooms.is_empty() {
                Block::default().borders(Borders::ALL).title(Span::styled("Horde Incoming? Rerun!", Style::default().fg(Color::Red)))
            } else {
                Block::default().borders(Borders::ALL).title(format!("{} Rooms | WASD/Arrows | ESC Quit | @ World({},{}) View({},{})", 
                    game_map.rooms.len(), state.player_x, state.player_y, view_x_min, view_y_min))
            };
            let map_widget = Paragraph::new(map_lines).block(map_title);
            f.render_widget(map_widget, chunks[0]);

            // RIGHT: Vertical HUD stack
            let hud_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints([
                    Constraint::Percentage(20),  // Mini-map
                    Constraint::Percentage(20),  // Backpack
                    Constraint::Percentage(20),  // Moodles
                    Constraint::Percentage(40),  // Dialogues/Messages
                ].as_ref())
                .split(chunks[1]);

            // Mini-map: Scaled 10x10 snippet
            let mini_w = 10;
            let mini_h = 10;
            let mut mini_lines = vec![];
            for my in 0..mini_h {
                let mut mini_line = vec![];
                for mx in 0..mini_w {
                    let m_x = state.player_x as i32 + (mx as i32 - 5);  // Center on player
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

            // Backpack: Simple table
            let backpack_items = state.inventory.iter().map(|item| vec![Span::styled(item, Style::default().fg(Color::Green))]).collect();
            let backpack_table = Table::new(backpack_items).block(Block::default().borders(Borders::ALL).title("Backpack")).widths(&[Constraint::Percentage(100)]);
            f.render_widget(backpack_table, hud_chunks[1]);

            // Moodles: Colored icons
            let moodle_lines = vec![Line::from(vec![
                Span::styled("Hunger:", Style::default().fg(Color::White)),
                Span::styled(format!("{:3}", state.hunger), Style::default().fg(if state.hunger < 50 { Color::Red } else { Color::Green })),
                Span::styled(" | Fatigue:", Style::default().fg(Color::White)),
                Span::styled(format!("{:3}", state.fatigue), Style::default().fg(if state.fatigue < 50 { Color::Red } else { Color::Green })),
            ])];
            let moodle_widget = Paragraph::new(moodle_lines).block(Block::default().borders(Borders::ALL).title("Moodles"));
            f.render_widget(moodle_widget, hud_chunks[2]);

            // Dialogues/Messages: Scrolling log
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
                // ... (Similar for s/a/d, add fatigue tick on move)
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

    println!("\nOutbreak contained—{} rooms survived. GG! Final hunger: {}", game_map.rooms.len(), state.hunger);
    Ok(())
}