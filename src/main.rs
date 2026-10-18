mod gfx;

use gfx::{Canvas, Color};
use macroquad::{
    audio::{PlaySoundParams, Sound, load_sound, play_sound, set_sound_volume},
    input::{
        KeyCode, MouseButton, is_key_pressed, is_mouse_button_down, is_mouse_button_pressed,
        mouse_position,
    },
    miniquad::date,
    rand::{gen_range, srand},
    time::get_time,
    window::{Conf, next_frame},
};
use std::time::Duration;

// --- Screen / playfield config ---------------------------------------------
const SCREEN_WIDTH: i32 = 800;
const SCREEN_HEIGHT: i32 = 1000;
const CELL_SIZE: i32 = 10;
const GAME_AREA_TOP: i32 = 200; // y where the playfield starts
const GAME_AREA_BOTTOM: i32 = 800; // y where the playfield ends (exclusive)
const CONTROL_ROW_TOP: i32 = 862;
const CONTROL_ROW_BOTTOM: i32 = 928;
const CONTROL_HEIGHT: i32 = 64;

const GRID_WIDTH: usize = (SCREEN_WIDTH / CELL_SIZE) as usize; // 80
const GRID_HEIGHT: usize = ((GAME_AREA_BOTTOM - GAME_AREA_TOP) / CELL_SIZE) as usize; // 60

const BASE_UPDATE_INTERVAL: Duration = Duration::from_millis(10);
const TOP_OUT_ROWS: usize = 4;

type Sand = Vec<Cell>;

// --- Core types --------------------------------------------------------------
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PieceColor {
    Red,
    // Coral,
    // Orange,
    // Yellow,
    Green,
    // Cyan,
    Blue,
    // Purple,
}

const ALL_COLORS: [PieceColor; 3] = [
    PieceColor::Red,
    // PieceColor::Coral,
    // PieceColor::Orange,
    // PieceColor::Yellow,
    PieceColor::Green,
    // PieceColor::Cyan,
    PieceColor::Blue,
    // PieceColor::Purple,
];

impl PieceColor {
    fn random() -> Self {
        ALL_COLORS[gen_range(0, ALL_COLORS.len())]
    }

    fn color(self) -> Color {
        match self {
            Self::Red => Color::new(255, 105, 97, 255),
            // Self::Coral => Color::new(255, 105, 97, 255),
            // Self::Orange => Color::new(255, 159, 67, 255),
            // Self::Yellow => Color::new(255, 220, 80, 255),
            Self::Green => Color::new(91, 207, 121, 255),
            // Self::Cyan => Color::new(66, 210, 214, 255),
            Self::Blue => Color::new(91, 145, 255, 255),
            // Self::Purple => Color::new(183, 115, 235, 255),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cell {
    Empty,
    Sand(PieceColor),
    /// An active falling shape. Falls like sand, but is tracked separately so it
    /// can be locked into place once it settles.
    Shape(PieceColor),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShapeKind {
    L,
    J,
    T,
    O,
    I,
    S,
    Z,
}

const ALL_SHAPES: [ShapeKind; 7] = [
    ShapeKind::L,
    ShapeKind::J,
    ShapeKind::T,
    ShapeKind::O,
    ShapeKind::I,
    ShapeKind::S,
    ShapeKind::Z,
];

impl ShapeKind {
    fn random() -> ShapeKind {
        ALL_SHAPES[gen_range(0, ALL_SHAPES.len())]
    }

    /// Offset cells of the shape from its top-left corner.
    fn cells(&self) -> Vec<(isize, isize)> {
        match self {
            ShapeKind::L => vec![
                (0, 0),
                (0, 1),
                (0, 2),
                (0, 3),
                (0, 4),
                (0, 5),
                (0, 6),
                (1, 6),
            ],
            ShapeKind::J => vec![
                (1, 0),
                (1, 1),
                (1, 2),
                (1, 3),
                (1, 4),
                (1, 5),
                (0, 6),
                (1, 6),
            ],
            ShapeKind::T => vec![(0, 0), (1, 0), (2, 0), (1, 1), (1, 2), (1, 3)],
            ShapeKind::O => vec![
                (0, 0),
                (1, 0),
                (2, 0),
                (0, 1),
                (1, 1),
                (2, 1),
                (0, 2),
                (1, 2),
                (2, 2),
            ],
            ShapeKind::I => vec![
                (0, 0),
                (0, 1),
                (0, 2),
                (0, 3),
                (0, 4),
                (0, 5),
                (0, 6),
                (0, 7),
            ],
            ShapeKind::S => vec![
                (1, 0),
                (2, 0),
                (1, 1),
                (2, 1),
                (0, 2),
                (1, 2),
                (0, 3),
                (1, 3),
            ],
            ShapeKind::Z => vec![
                (0, 0),
                (1, 0),
                (0, 1),
                (1, 1),
                (1, 2),
                (2, 2),
                (1, 3),
                (2, 3),
            ],
        }
    }

    fn rotated_cells(&self, turns: usize, scale: usize) -> Vec<(isize, isize)> {
        let mut cells = self.cells();
        for _ in 0..turns % 4 {
            let max_y = cells.iter().map(|(_, y)| *y).max().unwrap_or(0);
            for (x, y) in &mut cells {
                (*x, *y) = (max_y - *y, *x);
            }
        }

        let min_x = cells.iter().map(|(x, _)| *x).min().unwrap_or(0);
        let min_y = cells.iter().map(|(_, y)| *y).min().unwrap_or(0);
        for (x, y) in &mut cells {
            *x -= min_x;
            *y -= min_y;
        }

        let mut scaled = Vec::with_capacity(cells.len() * scale * scale);
        for (x, y) in cells {
            for dy in 0..scale as isize {
                for dx in 0..scale as isize {
                    scaled.push((x * scale as isize + dx, y * scale as isize + dy));
                }
            }
        }
        scaled
    }
}

struct Grid {
    width: usize,
    height: usize,
    cell_size: i32,
    origin_y: i32,
}

impl Grid {
    fn new() -> Self {
        Self {
            width: GRID_WIDTH,
            height: GRID_HEIGHT,
            cell_size: CELL_SIZE,
            origin_y: GAME_AREA_TOP,
        }
    }

    fn index(&self, x: usize, y: usize) -> usize {
        x + y * self.width
    }

    fn len(&self) -> usize {
        self.width * self.height
    }

    fn piece_scale(&self) -> usize {
        (self.width.min(self.height) / 20).clamp(1, 4)
    }

    fn in_bounds(&self, x: isize, y: isize) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height
    }
}

// --- Simulation ----------------------------------------------------------------
/// Advance the sand one step. Every grain moves at most once per step.
///
/// Per-grain rules:
///  1. fall straight down,
///  2. slide down a diagonal (random preferred side so piles scatter evenly).
fn step_sand(sand: &mut Sand, grid: &Grid, moved: &mut [bool]) {
    moved.fill(false);
    for y in (0..grid.height).rev() {
        // Alternate horizontal scan direction per row to avoid directional bias.
        if y % 2 == 0 {
            for x in 0..grid.width {
                step_cell(sand, grid, moved, x, y);
            }
        } else {
            for x in (0..grid.width).rev() {
                step_cell(sand, grid, moved, x, y);
            }
        }
    }
}

fn step_cell(sand: &mut Sand, grid: &Grid, moved: &mut [bool], x: usize, y: usize) {
    let idx = grid.index(x, y);
    if sand[idx] == Cell::Empty || moved[idx] {
        return;
    }

    // 1. Fall straight down.
    let below = idx + grid.width;
    if below < sand.len() && sand[below] == Cell::Empty {
        move_grain(sand, moved, idx, below);
        return;
    }

    // 2. Slide down a diagonal (both directions checked, random order).
    let pref: isize = if gen_range(0, 2) == 0 { 1 } else { -1 };
    for dx in [pref, -pref] {
        if let Some(target) = diag_target(grid, x, y, dx)
            && sand[target] == Cell::Empty
        {
            move_grain(sand, moved, idx, target);
            return;
        }
    }
}

fn move_grain(sand: &mut Sand, moved: &mut [bool], from: usize, to: usize) {
    sand[to] = sand[from];
    sand[from] = Cell::Empty;
    moved[to] = true;
}

fn diag_target(grid: &Grid, x: usize, y: usize, dx: isize) -> Option<usize> {
    let nx = x as isize + dx;
    let ny = y as isize + 1;
    grid.in_bounds(nx, ny)
        .then(|| grid.index(nx as usize, ny as usize))
}

/// Spawn a shape as active colored cells, horizontally centered on the
/// given aim column, near the top of the playfield. The shape is placed only
/// if every cell fits and the spawn area is clear.
fn spawn_shape(
    sand: &mut Sand,
    grid: &Grid,
    shape: ShapeKind,
    turns: usize,
    color: PieceColor,
    aim_x: usize,
) -> bool {
    let Some(positions) = spawn_positions(sand, grid, shape, turns, aim_x) else {
        return false;
    };
    for (x, y) in positions {
        sand[grid.index(x, y)] = Cell::Shape(color);
    }
    true
}

/// Grid cells a shape would occupy when dropped at `aim_x`, or `None` if it
/// doesn't fit or the spawn area is blocked.
fn spawn_positions(
    sand: &Sand,
    grid: &Grid,
    shape: ShapeKind,
    turns: usize,
    aim_x: usize,
) -> Option<Vec<(usize, usize)>> {
    let cells = shape.rotated_cells(turns, grid.piece_scale());
    let width = cells.iter().map(|(x, _)| *x).max().unwrap_or(0) as usize + 1;
    if width > grid.width {
        return None;
    }
    let left = aim_x.saturating_sub(width / 2).min(grid.width - width);

    let mut positions = Vec::with_capacity(cells.len());
    for (dx, dy) in cells {
        let (x, y) = (left as isize + dx, dy);
        if !grid.in_bounds(x, y) || sand[grid.index(x as usize, y as usize)] != Cell::Empty {
            return None;
        }
        positions.push((x as usize, y as usize));
    }
    Some(positions)
}

/// Whether the shape can be dropped anywhere on the screen in any rotation.
fn can_spawn_anywhere(sand: &Sand, grid: &Grid, shape: ShapeKind) -> bool {
    (0..4).any(|turns| {
        (0..grid.width).any(|aim_x| spawn_positions(sand, grid, shape, turns, aim_x).is_some())
    })
}

/// Whether a shape is currently falling / active on the field.
fn has_active_shape(sand: &Sand) -> bool {
    sand.iter().any(|cell| matches!(cell, Cell::Shape(_)))
}

/// True if the given cell could be moved by the simulation this tick.
fn cell_can_move(sand: &Sand, grid: &Grid, x: usize, y: usize) -> bool {
    let idx = grid.index(x, y);
    let below = idx + grid.width;
    if below < sand.len() && sand[below] == Cell::Empty {
        return true;
    }
    for dx in [-1isize, 1] {
        if let Some(target) = diag_target(grid, x, y, dx)
            && sand[target] == Cell::Empty
        {
            return true;
        }
    }
    false
}

/// If an active shape has come to rest (no cell can move anymore), convert it
/// into ordinary sand so the next shape can be spawned. Returns true if it
/// locked.
fn lock_settled_shape(sand: &mut Sand, grid: &Grid) -> bool {
    let mut found = false;
    let mut settled = true;
    for y in 0..grid.height {
        for x in 0..grid.width {
            if !matches!(sand[grid.index(x, y)], Cell::Shape(_)) {
                continue;
            }
            found = true;
            if cell_can_move(sand, grid, x, y) {
                settled = false;
            }
        }
    }
    if found && settled {
        for cell in sand.iter_mut() {
            if let Cell::Shape(color) = *cell {
                *cell = Cell::Sand(color);
            }
        }
    }
    found && settled
}

/// Destroys every group of same-colored sand (8-way connected) that touches
/// both the left and the right wall. Returns the number of grains destroyed.
fn clear_connected_colors(sand: &mut Sand, grid: &Grid) -> usize {
    let mut visited = vec![false; grid.len()];
    let mut destroyed = 0;

    for start_y in 0..grid.height {
        let start = grid.index(0, start_y);
        let Cell::Sand(color) = sand[start] else {
            continue;
        };
        if visited[start] {
            continue;
        }

        let mut group = vec![start];
        let mut stack = vec![(0usize, start_y)];
        let mut touches_right = false;
        visited[start] = true;

        while let Some((x, y)) = stack.pop() {
            touches_right |= x == grid.width - 1;
            for dy in -1isize..=1 {
                for dx in -1isize..=1 {
                    let (nx, ny) = (x as isize + dx, y as isize + dy);
                    if !grid.in_bounds(nx, ny) {
                        continue;
                    }
                    let idx = grid.index(nx as usize, ny as usize);
                    if !visited[idx] && sand[idx] == Cell::Sand(color) {
                        visited[idx] = true;
                        group.push(idx);
                        stack.push((nx as usize, ny as usize));
                    }
                }
            }
        }

        if touches_right {
            destroyed += group.len();
            for idx in group {
                sand[idx] = Cell::Empty;
            }
        }
    }
    destroyed
}

fn is_topped_out(sand: &Sand, grid: &Grid) -> bool {
    (0..TOP_OUT_ROWS.min(grid.height)).any(|y| {
        sand[grid.index(0, y)..grid.index(0, y) + grid.width]
            .iter()
            .any(|c| *c != Cell::Empty)
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GameState {
    Playing,
    Paused,
    GameOver,
}

struct Game {
    sand: Sand,
    moved: Vec<bool>,
    next_shape: ShapeKind,
    next_color: PieceColor,
    next_rotation: usize,
    aim_x: usize,
    score: u32,
    clears: u32,
    /// Set when a shape locks; cleared once the board has settled and been checked.
    pending_check: bool,
    state: GameState,
}

impl Game {
    fn new(grid: &Grid) -> Self {
        Self {
            sand: vec![Cell::Empty; grid.len()],
            moved: vec![false; grid.len()],
            next_shape: ShapeKind::random(),
            next_color: PieceColor::random(),
            next_rotation: 0,
            aim_x: grid.width / 2,
            score: 0,
            clears: 0,
            pending_check: false,
            state: GameState::Playing,
        }
    }

    fn drop_shape(&mut self, grid: &Grid) {
        if self.state != GameState::Playing || has_active_shape(&self.sand) {
            return;
        }
        // A blocked spawn is not game over; the player can aim elsewhere.
        if spawn_shape(
            &mut self.sand,
            grid,
            self.next_shape,
            self.next_rotation,
            self.next_color,
            self.aim_x,
        ) {
            self.next_shape = ShapeKind::random();
            self.next_color = PieceColor::random();
            self.next_rotation = 0;
        }
    }

    fn update(&mut self, grid: &Grid) {
        if self.state != GameState::Playing {
            return;
        }
        step_sand(&mut self.sand, grid, &mut self.moved);
        if lock_settled_shape(&mut self.sand, grid) {
            self.pending_check = true;
        }
        if has_active_shape(&self.sand) {
            return;
        }
        if !self.pending_check {
            if !can_spawn_anywhere(&self.sand, grid, self.next_shape) {
                self.state = GameState::GameOver;
            }
            return;
        }

        // Destroyed sand lets the pile above fall, which can connect new
        // groups, so keep checking until everything has settled.
        let destroyed = clear_connected_colors(&mut self.sand, grid);
        if destroyed > 0 {
            self.clears += 1;
            self.score += destroyed as u32 * (1 + self.clears / 10);
        } else if !self.moved.iter().any(|moved| *moved) {
            self.pending_check = false;
            if is_topped_out(&self.sand, grid) {
                self.state = GameState::GameOver;
            }
        }
    }
}

// --- Rendering ------------------------------------------------------------------
fn color_of(cell: Cell) -> Color {
    match cell {
        Cell::Empty => Color::BLACK,
        Cell::Sand(color) | Cell::Shape(color) => color.color(),
    }
}

fn draw_sand(sand: &Sand, grid: &Grid, d: &mut Canvas) {
    for y in 0..grid.height {
        for x in 0..grid.width {
            let cell = sand[grid.index(x, y)];
            if cell == Cell::Empty {
                continue;
            }
            let px = (x as i32) * grid.cell_size;
            let py = (y as i32) * grid.cell_size + grid.origin_y;
            d.draw_rectangle(px, py, grid.cell_size, grid.cell_size, color_of(cell));
        }
    }
}

fn draw_preview(shape: ShapeKind, turns: usize, color: PieceColor, grid: &Grid, d: &mut Canvas) {
    let cells = shape.rotated_cells(turns, grid.piece_scale());
    let cell_size = (12 / grid.piece_scale() as i32).max(3);
    for (x, y) in cells {
        d.draw_rectangle(
            620 + x as i32 * cell_size,
            82 + y as i32 * cell_size,
            cell_size - 1,
            cell_size - 1,
            color.color(),
        );
    }
}

fn hit_rect(x: i32, y: i32, rect_x: i32, rect_y: i32, width: i32, height: i32) -> bool {
    (rect_x..rect_x + width).contains(&x) && (rect_y..rect_y + height).contains(&y)
}

fn control_button_hit(x: i32, y: i32, index: usize) -> bool {
    let button_x = 12 + index as i32 * 196;
    hit_rect(x, y, button_x, CONTROL_ROW_TOP, 188, CONTROL_HEIGHT)
}

fn draw_control_button(label: &str, x: i32, y: i32, width: i32, height: i32, d: &mut Canvas) {
    d.draw_rectangle(x, y, width, height, Color::new(48, 57, 76, 255));
    d.draw_rectangle(x, y, width, 2, Color::new(102, 119, 153, 255));
    let text_x = x + (width - label.len() as i32 * 10) / 2;
    d.draw_text(label, text_x, y + (height - 20) / 2, 18, Color::RAYWHITE);
}

fn draw_game(game: &Game, grid: &Grid, d: &mut Canvas) {
    d.clear_background(Color::new(12, 15, 24, 255));
    d.draw_text("SANDTRIS", 32, 28, 38, Color::RAYWHITE);
    d.draw_text(
        &format!("SCORE  {:06}", game.score),
        34,
        88,
        24,
        Color::GOLD,
    );
    d.draw_text(
        &format!("CLEARS  {}", game.clears),
        34,
        126,
        20,
        Color::LIGHTGRAY,
    );
    d.draw_text(
        &format!("LEVEL  {}", game.clears / 10 + 1),
        252,
        126,
        20,
        Color::LIGHTGRAY,
    );
    d.draw_text("NEXT", 584, 42, 20, Color::LIGHTGRAY);
    draw_preview(
        game.next_shape,
        game.next_rotation,
        game.next_color,
        grid,
        d,
    );

    let aim_px = game.aim_x as i32 * grid.cell_size + grid.cell_size / 2;
    d.draw_line(
        aim_px,
        grid.origin_y,
        aim_px,
        grid.origin_y + grid.height as i32 * grid.cell_size,
        Color::new(100, 115, 145, 90),
    );
    draw_sand(&game.sand, grid, d);

    // Settled sand reaching this line ends the game.
    let danger_y = grid.origin_y + TOP_OUT_ROWS as i32 * grid.cell_size;
    d.draw_line(
        0,
        danger_y,
        grid.width as i32 * grid.cell_size,
        danger_y,
        Color::new(230, 70, 70, 200),
    );

    d.draw_text(
        "Tap to drop in a column; connect colors wall to wall to clear.",
        32,
        838,
        17,
        Color::GRAY,
    );

    if matches!(game.state, GameState::Paused | GameState::GameOver) {
        d.draw_rectangle(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT, Color::new(0, 0, 0, 190));
        let (title, subtitle) = if game.state == GameState::Paused {
            ("PAUSED", "Press P or tap RESUME")
        } else {
            ("GAME OVER", "Press Enter or tap PLAY AGAIN")
        };
        d.draw_text(title, 250, 420, 48, Color::RAYWHITE);
        d.draw_text(subtitle, 230, 482, 22, Color::LIGHTGRAY);
        if game.state == GameState::GameOver {
            d.draw_text(
                &format!("Final score: {}", game.score),
                286,
                526,
                20,
                Color::GOLD,
            );
        }
    }
    for (index, label) in ["LEFT", "RIGHT", "ROTATE", "DROP"].iter().enumerate() {
        draw_control_button(
            label,
            12 + index as i32 * 196,
            CONTROL_ROW_TOP,
            188,
            CONTROL_HEIGHT,
            d,
        );
    }
    let action_label = match game.state {
        GameState::Paused => "RESUME",
        GameState::GameOver => "PLAY AGAIN",
        GameState::Playing => "PAUSE",
    };
    draw_control_button(
        action_label,
        206,
        CONTROL_ROW_BOTTOM,
        188,
        CONTROL_HEIGHT,
        d,
    );
    draw_control_button("MENU", 406, CONTROL_ROW_BOTTOM, 188, CONTROL_HEIGHT, d);
}

// --- Audio ------------------------------------------------------------------------
// Browsers can't list directories, so the tracks are listed explicitly. They are
// OGG because that is the compressed format macroquad can play on every target.
const MUSIC_TRACKS: [&str; 4] = [
    "assets/music/sandtris_a.ogg",
    "assets/music/sandtris_b.ogg",
    "assets/music/sandtris_c.ogg",
    "assets/music/sandtris_d.ogg",
];

/// Loads one randomly chosen track. Audio is optional: on failure the game
/// simply runs silently.
async fn load_random_track() -> Option<Sound> {
    let path = MUSIC_TRACKS[gen_range(0, MUSIC_TRACKS.len())];
    match load_sound(path).await {
        Ok(sound) => Some(sound),
        Err(err) => {
            eprintln!("Could not load music {path}: {err}");
            None
        }
    }
}

// --- Menu -------------------------------------------------------------------------
const DEFAULT_VOLUME: f32 = 0.5;
const VOLUME_STEP: f32 = 0.1;
const SLIDER_X: i32 = 200;
const SLIDER_Y: i32 = 500;
const SLIDER_W: i32 = 400;
const SLIDER_H: i32 = 16;

/// Moves the volume one step up (`dir > 0`) or down, snapped to the step size.
fn step_volume(volume: f32, dir: i32) -> f32 {
    let stepped = volume + dir.signum() as f32 * VOLUME_STEP;
    ((stepped / VOLUME_STEP).round() * VOLUME_STEP).clamp(0.0, 1.0)
}

fn volume_from_mouse_x(x: i32) -> f32 {
    ((x - SLIDER_X) as f32 / SLIDER_W as f32).clamp(0.0, 1.0)
}

/// Whether a point is on the slider (with some vertical slack for easier grabbing).
fn slider_hit(x: i32, y: i32) -> bool {
    (SLIDER_X - 10..=SLIDER_X + SLIDER_W + 10).contains(&x)
        && (SLIDER_Y - 14..=SLIDER_Y + SLIDER_H + 14).contains(&y)
}

fn draw_menu(volume: f32, d: &mut Canvas) {
    d.draw_rectangle(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT, Color::new(0, 0, 0, 215));
    d.draw_text("MENU", 322, 380, 48, Color::RAYWHITE);
    d.draw_text(
        &format!("Music volume: {}%", (volume * 100.0).round() as i32),
        SLIDER_X,
        SLIDER_Y - 40,
        24,
        Color::GOLD,
    );

    d.draw_rectangle(SLIDER_X, SLIDER_Y, SLIDER_W, SLIDER_H, Color::DARKGRAY);
    let fill = (volume * SLIDER_W as f32) as i32;
    d.draw_rectangle(SLIDER_X, SLIDER_Y, fill, SLIDER_H, Color::GOLD);
    d.draw_circle(
        SLIDER_X + fill,
        SLIDER_Y + SLIDER_H / 2,
        14.0,
        Color::RAYWHITE,
    );

    d.draw_text(
        "Drag the slider to change volume",
        SLIDER_X,
        SLIDER_Y + 60,
        20,
        Color::LIGHTGRAY,
    );
    draw_control_button("CLOSE", 306, 640, 188, CONTROL_HEIGHT, d);
}

// --- Main -------------------------------------------------------------------------
fn window_conf() -> Conf {
    Conf {
        window_title: "Sandtris".to_owned(),
        window_width: SCREEN_WIDTH,
        window_height: SCREEN_HEIGHT,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    srand(date::now() as u64);
    // Native builds resolve `assets/` against the project root, like the web
    // build resolves it against the page.
    #[cfg(not(target_arch = "wasm32"))]
    macroquad::file::set_pc_assets_folder(env!("CARGO_MANIFEST_DIR"));

    let grid = Grid::new();
    let mut game = Game::new(&grid);
    let mut d = Canvas;

    let music = load_random_track().await;
    let mut volume = DEFAULT_VOLUME;
    // Last volume sent to the mixer; paused games are muted rather than stopped.
    let mut applied_volume = volume;
    if let Some(music) = &music {
        play_sound(
            music,
            PlaySoundParams {
                looped: true,
                volume,
            },
        );
    }

    let mut menu_open = false;
    let mut last_update = get_time();

    loop {
        #[cfg(not(target_arch = "wasm32"))]
        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        let level = game.clears / 10;
        let update_interval = BASE_UPDATE_INTERVAL
            .saturating_sub(Duration::from_millis(2 * level as u64))
            .max(Duration::from_millis(8))
            .as_secs_f64();
        let (mouse_x, mouse_y) = {
            let (x, y) = mouse_position();
            (x as i32, y as i32)
        };
        let clicked = is_mouse_button_pressed(MouseButton::Left);

        // The menu freezes the game but keeps the music playing so the volume
        // change is audible. It can't be opened while paused (music is muted).
        if (is_key_pressed(KeyCode::M)
            || clicked
                && hit_rect(
                    mouse_x,
                    mouse_y,
                    406,
                    CONTROL_ROW_BOTTOM,
                    188,
                    CONTROL_HEIGHT,
                ))
            && game.state != GameState::Paused
        {
            menu_open = !menu_open;
            last_update = get_time();
        }

        if menu_open {
            let mut new_volume = volume;
            if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A) {
                new_volume = step_volume(volume, -1);
            }
            if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D) {
                new_volume = step_volume(volume, 1);
            }
            if is_mouse_button_down(MouseButton::Left) && slider_hit(mouse_x, mouse_y) {
                new_volume = volume_from_mouse_x(mouse_x);
            }
            volume = new_volume;
            if clicked && hit_rect(mouse_x, mouse_y, 306, 640, 188, CONTROL_HEIGHT) {
                menu_open = false;
            }
        } else if game.state == GameState::Playing {
            if is_key_pressed(KeyCode::Left)
                || is_key_pressed(KeyCode::A)
                || clicked && control_button_hit(mouse_x, mouse_y, 0)
            {
                game.aim_x = game.aim_x.saturating_sub(1);
            }
            if is_key_pressed(KeyCode::Right)
                || is_key_pressed(KeyCode::D)
                || clicked && control_button_hit(mouse_x, mouse_y, 1)
            {
                game.aim_x = (game.aim_x + 1).min(grid.width - 1);
            }
            if is_key_pressed(KeyCode::Up)
                || is_key_pressed(KeyCode::W)
                || clicked && control_button_hit(mouse_x, mouse_y, 2)
            {
                game.next_rotation = (game.next_rotation + 1) % 4;
            }
            if is_key_pressed(KeyCode::Space) || clicked && control_button_hit(mouse_x, mouse_y, 3)
            {
                game.drop_shape(&grid);
            }
            if clicked
                && mouse_y >= grid.origin_y
                && mouse_y < grid.origin_y + grid.height as i32 * grid.cell_size
            {
                game.aim_x = (mouse_x.clamp(0, SCREEN_WIDTH - 1) / grid.cell_size) as usize;
                game.drop_shape(&grid);
            }
            if is_key_pressed(KeyCode::P)
                || clicked
                    && hit_rect(
                        mouse_x,
                        mouse_y,
                        206,
                        CONTROL_ROW_BOTTOM,
                        188,
                        CONTROL_HEIGHT,
                    )
            {
                game.state = GameState::Paused;
            }
            if get_time() - last_update >= update_interval {
                game.update(&grid);
                last_update = get_time();
            }
        } else if game.state == GameState::Paused {
            if is_key_pressed(KeyCode::P)
                || clicked
                    && hit_rect(
                        mouse_x,
                        mouse_y,
                        206,
                        CONTROL_ROW_BOTTOM,
                        188,
                        CONTROL_HEIGHT,
                    )
            {
                game.state = GameState::Playing;
                last_update = get_time();
            }
        } else if is_key_pressed(KeyCode::Enter)
            || clicked
                && hit_rect(
                    mouse_x,
                    mouse_y,
                    206,
                    CONTROL_ROW_BOTTOM,
                    188,
                    CONTROL_HEIGHT,
                )
        {
            game = Game::new(&grid);
            last_update = get_time();
        }

        let target_volume = if game.state == GameState::Paused {
            0.0
        } else {
            volume
        };
        if target_volume != applied_volume {
            applied_volume = target_volume;
            if let Some(music) = &music {
                set_sound_volume(music, target_volume);
            }
        }

        draw_game(&game, &grid, &mut d);
        if menu_open {
            draw_menu(volume, &mut d);
        }
        next_frame().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Lcg(u64);

    impl Lcg {
        fn new(seed: u64) -> Self {
            Self(seed)
        }
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0 >> 33
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    fn grains(sand: &Sand) -> usize {
        sand.iter().filter(|c| **c != Cell::Empty).count()
    }

    fn lowest_grain_y(sand: &Sand, grid: &Grid) -> Option<usize> {
        for y in (0..grid.height).rev() {
            for x in 0..grid.width {
                if sand[grid.index(x, y)] != Cell::Empty {
                    return Some(y);
                }
            }
        }
        None
    }

    fn pile_width(sand: &Sand, grid: &Grid) -> usize {
        let mut min_x = grid.width;
        let mut max_x = 0;
        for y in 0..grid.height {
            for x in 0..grid.width {
                if sand[grid.index(x, y)] != Cell::Empty {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                }
            }
        }
        if min_x > max_x { 0 } else { max_x - min_x + 1 }
    }

    #[test]
    fn conserves_grains_and_stays_in_bounds() {
        let grid = Grid::new();
        let mut sand = vec![Cell::Empty; grid.len()];
        let mut moved = vec![false; grid.len()];
        let mut rng = Lcg::new(0xC0FFEE);

        for _ in 0..500 {
            sand[grid.index(rng.below(grid.width), rng.below(grid.height))] =
                Cell::Sand(PieceColor::Green);
        }
        let total = grains(&sand);

        for _ in 0..3000 {
            step_sand(&mut sand, &grid, &mut moved);
            assert_eq!(grains(&sand), total);
        }
    }

    #[test]
    fn falls_until_resting_on_floor() {
        let grid = Grid::new();
        let mut sand = vec![Cell::Empty; grid.len()];
        let mut moved = vec![false; grid.len()];
        let total = 50;

        for i in 0..total {
            sand[grid.index(40, i)] = Cell::Sand(PieceColor::Green);
        }

        for _ in 0..grid.height + 50 {
            step_sand(&mut sand, &grid, &mut moved);
        }

        assert_eq!(grains(&sand), total);
        assert_eq!(lowest_grain_y(&sand, &grid), Some(grid.height - 1));
        assert!(sand[grid.index(40, grid.height - 1)] != Cell::Empty);
    }

    #[test]
    fn poured_column_scatters_into_a_pile() {
        let grid = Grid::new();
        let mut sand = vec![Cell::Empty; grid.len()];
        let mut moved = vec![false; grid.len()];

        // Drop a continuous stream down the middle of the board.
        for _ in 0..400 {
            for _ in 0..4 {
                sand[grid.index(40, 0)] = Cell::Sand(PieceColor::Green);
                step_sand(&mut sand, &grid, &mut moved);
            }
        }

        assert_eq!(grains(&sand), 1600);
        let width = pile_width(&sand, &grid);
        assert!(
            width > 1,
            "sand should spread sideways, pile width was {width}"
        );
    }

    #[test]
    fn shape_lands_inside_the_window() {
        let grid = Grid::new();
        let mut sand = vec![Cell::Empty; grid.len()];
        let mut moved = vec![false; grid.len()];

        // Exactly what the game does on a left-click: spawn an L shape at the
        // mouse column, then simulate.
        assert!(spawn_shape(
            &mut sand,
            &grid,
            ShapeKind::L,
            0,
            PieceColor::Coral,
            40
        ));
        for _ in 0..300 {
            step_sand(&mut sand, &grid, &mut moved);
        }

        assert_eq!(
            grains(&sand),
            ShapeKind::L.rotated_cells(0, grid.piece_scale()).len()
        );
        // The pile must rest on the floor row (drawn at GAME_AREA_BOTTOM - cell
        // size), well above the window's bottom edge, never outside the grid.
        assert_eq!(lowest_grain_y(&sand, &grid), Some(grid.height - 1));
        assert!(pile_width(&sand, &grid) > 1, "shape sand should scatter");
    }

    #[test]
    fn shape_locks_when_settled_then_allows_new_spawn() {
        let grid = Grid::new();
        let mut sand = vec![Cell::Empty; grid.len()];
        let mut moved = vec![false; grid.len()];

        assert!(spawn_shape(
            &mut sand,
            &grid,
            ShapeKind::L,
            0,
            PieceColor::Coral,
            40
        ));
        assert!(
            has_active_shape(&sand),
            "shape should be active after spawn"
        );
        assert!(
            sand.contains(&Cell::Shape(PieceColor::Coral)),
            "spawned shape should use its assigned color"
        );

        // While the shape is still falling it must NOT lock.
        for _ in 0..30 {
            step_sand(&mut sand, &grid, &mut moved);
            lock_settled_shape(&mut sand, &grid);
            assert!(
                has_active_shape(&sand),
                "shape is still falling, must not lock"
            );
        }

        // Once it has fallen and settled it locks into ordinary sand.
        for _ in 0..grid.height * 2 {
            step_sand(&mut sand, &grid, &mut moved);
            lock_settled_shape(&mut sand, &grid);
        }
        assert!(!has_active_shape(&sand), "shape should have locked");
        assert_eq!(
            grains(&sand),
            ShapeKind::L.rotated_cells(0, grid.piece_scale()).len()
        );
        assert!(sand.contains(&Cell::Sand(PieceColor::Coral)));
        assert!(!sand.contains(&Cell::Sand(PieceColor::Green)));

        // The lock frees the spawn gate: the next shape can be dropped.
        assert!(spawn_shape(
            &mut sand,
            &grid,
            ShapeKind::L,
            0,
            PieceColor::Coral,
            40
        ));
        assert!(has_active_shape(&sand));
    }

    #[test]
    fn all_shapes_fall_and_lock() {
        let grid = Grid::new();
        for shape in ALL_SHAPES {
            let mut sand = vec![Cell::Empty; grid.len()];
            let mut moved = vec![false; grid.len()];
            let count = shape.rotated_cells(0, grid.piece_scale()).len();

            assert!(
                spawn_shape(&mut sand, &grid, shape, 0, PieceColor::Coral, 40),
                "{shape:?} should spawn"
            );
            for _ in 0..grid.height * 2 {
                step_sand(&mut sand, &grid, &mut moved);
                lock_settled_shape(&mut sand, &grid);
            }

            assert!(
                !has_active_shape(&sand),
                "{shape:?} should lock after falling"
            );
            assert_eq!(grains(&sand), count, "{shape:?} lost grains");
            assert_eq!(lowest_grain_y(&sand, &grid), Some(grid.height - 1));
        }
    }

    #[test]
    fn failed_spawn_does_not_partially_place_shape() {
        let grid = Grid::new();
        let mut sand = vec![Cell::Empty; grid.len()];
        let blocker = grid.index(40, 1);
        sand[blocker] = Cell::Sand(PieceColor::Green);
        let before = sand.clone();

        assert!(!spawn_shape(
            &mut sand,
            &grid,
            ShapeKind::T,
            0,
            PieceColor::Coral,
            40
        ));
        assert_eq!(sand, before);
    }

    #[test]
    fn rotated_shapes_keep_all_cells_inside_the_grid() {
        let grid = Grid::new();
        for shape in ALL_SHAPES {
            for turns in 0..4 {
                let cells = shape.rotated_cells(turns, grid.piece_scale());
                assert_eq!(
                    cells.len(),
                    shape.cells().len() * grid.piece_scale() * grid.piece_scale()
                );
                let mut sand = vec![Cell::Empty; grid.len()];
                assert!(spawn_shape(
                    &mut sand,
                    &grid,
                    shape,
                    turns,
                    PieceColor::Coral,
                    0
                ));
                assert_eq!(grains(&sand), cells.len());
            }
        }
    }

    #[test]
    fn piece_scale_tracks_playfield_dimensions() {
        let mut grid = Grid::new();
        assert_eq!(grid.piece_scale(), 3);

        grid.width = 40;
        grid.height = 40;
        assert_eq!(grid.piece_scale(), 2);

        grid.width = 120;
        grid.height = 100;
        assert_eq!(grid.piece_scale(), 4);
    }

    #[test]
    fn volume_steps_stay_in_range_and_avoid_float_drift() {
        let mut volume = 0.0;
        for _ in 0..3 {
            volume = step_volume(volume, 1);
        }
        assert_eq!(volume, 0.3);
        for _ in 0..20 {
            volume = step_volume(volume, 1);
        }
        assert_eq!(volume, 1.0);
        for _ in 0..20 {
            volume = step_volume(volume, -1);
        }
        assert_eq!(volume, 0.0);
    }

    #[test]
    fn slider_maps_mouse_position_to_volume() {
        assert_eq!(volume_from_mouse_x(SLIDER_X - 50), 0.0);
        assert_eq!(volume_from_mouse_x(SLIDER_X + SLIDER_W / 2), 0.5);
        assert_eq!(volume_from_mouse_x(SLIDER_X + SLIDER_W + 50), 1.0);
        assert!(slider_hit(SLIDER_X + 10, SLIDER_Y + 5));
        assert!(!slider_hit(SLIDER_X + 10, SLIDER_Y + 100));
    }

    #[test]
    fn blocked_spawn_is_not_game_over() {
        let grid = Grid::new();
        let mut game = Game::new(&grid);
        // Settled sand below the top-out rows, but blocking the spawn column.
        for x in 0..grid.width {
            game.sand[grid.index(x, TOP_OUT_ROWS + 1)] = Cell::Sand(PieceColor::Green);
        }
        let before = game.sand.clone();

        game.drop_shape(&grid);

        assert_eq!(game.state, GameState::Playing);
        assert_eq!(game.sand, before);
    }

    #[test]
    fn game_over_when_piece_fits_nowhere() {
        let grid = Grid::new();
        let mut game = Game::new(&grid);
        // Fills everything below the top-out rows: nothing can fall, nothing fits.
        for y in TOP_OUT_ROWS + 1..grid.height {
            for x in 0..grid.width {
                game.sand[grid.index(x, y)] = Cell::Sand(PieceColor::Green);
            }
        }
        assert!(!can_spawn_anywhere(&game.sand, &grid, game.next_shape));

        game.update(&grid);

        assert_eq!(game.state, GameState::GameOver);
    }

    #[test]
    fn not_game_over_while_some_spot_still_fits() {
        let grid = Grid::new();
        let mut game = Game::new(&grid);
        // Leaves the left edge free so any shape can still be dropped there.
        for x in 20..grid.width {
            game.sand[grid.index(x, TOP_OUT_ROWS + 1)] = Cell::Sand(PieceColor::Green);
        }
        assert!(can_spawn_anywhere(&game.sand, &grid, game.next_shape));

        game.update(&grid);

        assert_eq!(game.state, GameState::Playing);
    }

    #[test]
    fn settled_sand_reaching_the_top_is_game_over() {
        let grid = Grid::new();
        let mut game = Game::new(&grid);
        // A packed board with alternating colors, so nothing settles or clears.
        for y in TOP_OUT_ROWS - 1..grid.height {
            for x in 0..grid.width {
                game.sand[grid.index(x, y)] = Cell::Sand(if x % 2 == 0 {
                    PieceColor::Green
                } else {
                    PieceColor::Blue
                });
            }
        }
        game.pending_check = true;

        game.update(&grid);

        assert_eq!(game.state, GameState::GameOver);
    }

    #[test]
    fn falling_shape_at_the_top_is_not_game_over() {
        let grid = Grid::new();
        let mut game = Game::new(&grid);
        game.drop_shape(&grid);
        assert!(has_active_shape(&game.sand));

        game.update(&grid);

        assert_eq!(game.state, GameState::Playing);
    }

    #[test]
    fn destroys_same_color_chain_touching_both_walls() {
        let grid = Grid::new();
        let mut sand = vec![Cell::Empty; grid.len()];
        let last = grid.height - 1;
        let green = Cell::Sand(PieceColor::Green);
        // A bottom row plus a diagonal step, all one color.
        for x in 0..grid.width - 1 {
            sand[grid.index(x, last)] = green;
        }
        sand[grid.index(grid.width - 1, last - 1)] = green;
        let bystander = grid.index(10, last - 1);
        sand[bystander] = Cell::Sand(PieceColor::Coral);

        assert_eq!(clear_connected_colors(&mut sand, &grid), grid.width);
        assert_eq!(grains(&sand), 1);
        assert_eq!(sand[bystander], Cell::Sand(PieceColor::Coral));
    }

    #[test]
    fn keeps_chains_that_mix_colors_or_miss_a_wall() {
        let grid = Grid::new();
        let last = grid.height - 1;

        let mut mixed = vec![Cell::Empty; grid.len()];
        for x in 0..grid.width {
            mixed[grid.index(x, last)] = Cell::Sand(if x == 40 {
                PieceColor::Blue
            } else {
                PieceColor::Green
            });
        }
        assert_eq!(clear_connected_colors(&mut mixed, &grid), 0);

        let mut short = vec![Cell::Empty; grid.len()];
        for x in 0..grid.width - 1 {
            short[grid.index(x, last)] = Cell::Sand(PieceColor::Green);
        }
        assert_eq!(clear_connected_colors(&mut short, &grid), 0);
    }

    #[test]
    fn destroyed_chain_lets_the_pile_above_fall() {
        let grid = Grid::new();
        let mut game = Game::new(&grid);
        let last = grid.height - 1;
        for x in 0..grid.width {
            game.sand[grid.index(x, last)] = Cell::Sand(PieceColor::Green);
        }
        game.sand[grid.index(40, last - 1)] = Cell::Sand(PieceColor::Coral);
        game.pending_check = true;

        for _ in 0..5 {
            game.update(&grid);
        }

        assert_eq!(game.clears, 1);
        assert_eq!(game.score, grid.width as u32);
        assert_eq!(grains(&game.sand), 1);
        assert_eq!(
            game.sand[grid.index(40, last)],
            Cell::Sand(PieceColor::Coral)
        );
        assert_eq!(game.state, GameState::Playing);
    }
}
