use std::vec::Vec;
use raylib::prelude::*;

type SandGrid = Vec<Vec<Cell>>;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Cell {
    Empty,
    Full,
}


fn draw_grid(grain_size: usize, width: i32, height: i32, d: &mut RaylibDrawHandle<'_>)
{
    let color = Color::WHITESMOKE;
    // Draw vertical lines
    for x in (0..=width).step_by(grain_size) {
        d.draw_line(x, 0, x, height, color);
    }

    // Draw horizontal lines
    for y in (0..=height).step_by(grain_size) {
        d.draw_line(0, y, width, y, color);
    }
}

fn update_sand(sand: &mut SandGrid)
{
    let length = sand.len();
    for x in 0..sand[length-1].len()
    {
        if sand[length-2][x] == Cell::Empty { continue; }
        if sand[length-1][x] != Cell::Empty { continue; }
        sand[length-1][x] = sand[length-2][x];
        sand[length-2][x] = Cell::Empty;
    }

    for y in (1..sand.len()-1).rev()
    {
        for x in 0..sand[y].len()
        {
            if sand[y-1][x] == Cell::Empty { continue; }

            if sand[y][x] != Cell::Empty {

                if x <= 1 || x >= sand[y].len()-2
                {
                    continue;
                }
                let left_ok = sand[y][x-1] == Cell::Empty;
                let right_ok = sand[y][x+1] == Cell::Empty;
                if !(left_ok || right_ok)
                {  continue; }
                if left_ok && right_ok
                {
                    print!("test");
                }
                if left_ok
                {
                    sand[y-1][x] = Cell::Empty;
                    sand[y][x-1] = Cell::Full;
                } else if right_ok
                {
                    sand[y-1][x] = Cell::Empty;
                    sand[y][x+1] = Cell::Full;
                }
            }

            sand[y-1][x] = Cell::Empty;
            sand[y][x] = Cell::Full;
        }
    }

}

fn draw_sand(sand: &mut SandGrid, d: &mut RaylibDrawHandle, grain_size: usize)
{
    let gs: i32 = grain_size as i32;
    for y in 0..sand.len()
    {
        for x in 0..sand[y].len()
        {
            if sand[y][x] != Cell::Empty
            {
                d.draw_rectangle((x*grain_size) as i32, (y*grain_size) as i32, gs, gs, Color::RED);
            }
        }
    }
}

fn get_count(sand: &SandGrid) -> usize {
    sand.iter()
        .flatten()
        .filter(|&&cell| cell != Cell::Empty)
        .count()
}


fn main() {

    let (screen_width, screen_height) = (500, 500);
    let grain_size = 5;
    let grid_shape: (usize, usize) = (screen_height / grain_size, screen_width / grain_size);

    let mut sand: SandGrid = vec![vec![Cell::Empty; grid_shape.1]; grid_shape.0];
    println!("sand id at start: {:p}", &sand);

    // Large "S" shape
    let offset_x = 50;
    let offset_y = 5;
    for y in 0..10 {
        for x in 0..15 {
            // Top horizontal bar
            if y < 5 {
                if x < 5 { continue; }
                sand[y+offset_y][offset_x + x] = Cell::Full;
            }
            // Bottom horizontal bar
            else 
            {
                if x > 9 { continue; }
                sand[y + offset_y][offset_x + x] = Cell::Full;
            }
        }
    }

    // Create a window: width, height, title
    let (mut rl, thread) = raylib::init()
        .size(screen_width as i32, screen_height as i32)
        .title("Hello, Raylib in Rust!")
        .build();

    let mut should_update = false;
    let mut grid_on = false;

    // Main game loop
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

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::DARKGRAY);
        if should_update { update_sand(&mut sand); }
        
        draw_sand(&mut sand, &mut d, grain_size);
        if grid_on {
            draw_grid(grain_size, screen_width as i32, screen_height as i32, &mut d);
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
