mod types;

use std::vec;
use std::cmp;

use rand::random;
use rand::random_bool;
use rand::random_range;
use raylib::prelude::*;
use types::*;

use rand::{Rng};


// Define special colors
static BACKGROUND_COLOR: Color = Color::BLACK;
static EMPTY_COLOR: Color = BACKGROUND_COLOR;
static GRID_COLOR: Color = Color::new(60, 60, 60, 60);

static GRID_ON: bool = true;
static SCREEN_SIZE: (usize, usize) = (1920/2, 1200/2);
static FULLSCREEN: bool = false;


fn draw_grid(grain_size: usize, width: i32, height: i32, color: Color, d: &mut RaylibDrawHandle<'_>)
{
    // Draw vertical lines
    for x in (0..=width).step_by(grain_size) {
        d.draw_line(x, 0, x, height, color);
    }

    // Draw horizontal lines
    for y in (0..=height).step_by(grain_size) {
        d.draw_line(0, y, width, y, color);
    }
}

fn update_sand(sand: &mut types::SandGrid)
{
    let length = sand.len();
    for x in 0..sand[length-1].len()
    {
        if !sand[length-2][x].full { continue; }
        if sand[length-1][x].full { continue; }
        sand[length-1][x] = sand[length-2][x];
        sand[length-2][x].full = false;
    }

    for y in (1..sand.len()-1).rev()
    {
        for x in 0..sand[y].len()
        {
            if !sand[y-1][x].full { continue; }

            if sand[y][x].full {

                if x <= 1 || x >= sand[y].len()-2
                {
                    continue;
                }
                let mut left_ok = !sand[y][x-1].full;
                let right_ok = !sand[y][x+1].full;
                if !(left_ok || right_ok)
                {  continue; }
                if left_ok && right_ok
                {
                    if (rand::random_bool(1.0 / 2.0))
                    {
                        left_ok = false;
                    };
                }
                if left_ok
                {
                    sand[y-1][x].full = false;
                    sand[y][x-1].full = true;
                    sand[y][x-1].color = sand[y-1][x].color;
                } else if right_ok
                {
                    sand[y-1][x].full = false;
                    sand[y][x+1].full = true;
                    sand[y][x+1].color = sand[y-1][x].color;
                }
            }

            sand[y-1][x].full = false;
            sand[y][x].full = true;
            sand[y][x].color = sand[y-1][x].color;
        }
    }

}

fn draw_sand(sand: &mut types::SandGrid, d: &mut RaylibDrawHandle, grain_size: usize)
{
    let gs: i32 = grain_size as i32;
    for y in 0..sand.len()
    {
        for x in 0..sand[y].len()
        {
            if sand[y][x].full
            {
                d.draw_rectangle((x*grain_size) as i32, (y*grain_size) as i32, gs, gs, sand[y][x].color);
            }
        }
    }
}


fn put_shape(shape: ShapeType, position: Vec<usize>, color: Color, sand: &mut SandGrid, variated: bool)
{
    match shape
    {
        // ShapeType::L => put_l(&position, sand),
        ShapeType::S => put_s(&position, color, sand, variated),
        // ShapeType::I => put_i(&position, sand),
        _ => panic!("Shape not implemented")
    }
}

fn put_s(position: &Vec<usize>, color: Color, sand: &mut SandGrid, variated: bool)
{

    let o_x = position[0];
    let o_y = position[1];
    let mut variance_coeff = 0.0;
    
    for y in 0..10 {
        for x in 0..15 {
            // Top horizontal bar
            if variated 
            {
                if rand::random_range(0.0..=1.0) < 0.05
                {
                    variance_coeff = 0.2;
                } else {
                    variance_coeff = 0.0;
                }
            }
            if y < 5 {
                if x < 5 { continue; }
                sand[o_y + y][o_x  + x] = Grain::new(color.brightness(-variance_coeff), true);
            }
            // Bottom horizontal bar
            else
            {
                if x > 9 { continue; }
                sand[o_y + y][o_x  + x] = Grain::new(color.brightness(-variance_coeff), true);
            }
        }
    }
}

fn init_sand(sand: &mut types::SandGrid, grid_shape: &(usize, usize))
{
    for y in 0..grid_shape.0
    {
        for x in 0..grid_shape.1
        {
            sand[y][x] = Grain::new(Color::WHITE, false);
        }
    }
}

fn get_count(sand: &types::SandGrid) -> usize
{
    sand.iter()
        .flatten()
        .filter(|&&grain| grain.full)
        .count()
}

fn quantize_position(position: &(usize, usize), grain_size: &usize) -> (usize, usize)
{
    //clamp position
    let x = cmp::max(position.0, 0);
    let y = cmp::max(position.1, 0);

    (0, 0)
}

fn main() {

    let (screen_width, screen_height) = SCREEN_SIZE;
    let grain_size = 5;
    let grid_shape: (usize, usize) = (screen_height / grain_size, screen_width / grain_size);

    let mut sand: types::SandGrid = vec![vec![Grain { color: Color::WHITE, full: false }; grid_shape.1]; grid_shape.0];
    init_sand(&mut sand, &grid_shape);
    put_shape(ShapeType::S, vec!(90, 5), Color::GREEN, &mut sand, true);

    // Create a window: width, height, title
    let (mut rl, thread) = raylib::init()
    .size(screen_width as i32, screen_height as i32)
    .title("SANDTRIS")
    .build();

    if !rl.is_window_fullscreen() && FULLSCREEN
    {
        rl.toggle_fullscreen();
    }

    let mut should_update = false;
    let mut grid_on = GRID_ON;

    // Main game loop
    let ups = 200;
    let frame_delay = std::time::Duration::from_millis(1000 / ups);
    let mut last_update_time = std::time::Instant::now();
    while !rl.window_should_close() {

        if rl.is_key_released(KeyboardKey::KEY_SPACE)
        {
            should_update = !should_update;
        }
        if rl.is_key_released(KeyboardKey::KEY_G)
        {
            grid_on = !grid_on;
        }
        if rl.is_key_released(KeyboardKey::KEY_DOWN)
        {
            update_sand(&mut sand);
        }

        if rl.is_key_pressed(KeyboardKey::KEY_F)
        {
            rl.toggle_fullscreen();
        }


        // println!("{:?}", rl.get_mouse_position());

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(BACKGROUND_COLOR);

        // Update sand if needed.
        if std::time::Instant::now() - last_update_time > frame_delay
        {
            if should_update {
                update_sand(&mut sand);
            }
            last_update_time = std::time::Instant::now();
        };

        draw_sand(&mut sand, &mut d, grain_size);
        if grid_on {
            draw_grid(grain_size, screen_width as i32, screen_height as i32, GRID_COLOR, &mut d);
        }

        // Draw sand count text
        let count = get_count(&sand);
        d.draw_text(
            &format!("Sand Count: {}", count),
            10, 10, // x, y position
            20,     // font size
            Color::WHEAT,
        );
    }
}
