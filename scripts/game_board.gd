class_name GameBoard
extends Reference

const WIDTH := 80
const HEIGHT := 128
const EMPTY := -1
const PARTICLE_SCALE := 8
const COMBO_WINDOW_SECONDS := 1.4
const SPAWN_COLUMN := 40

const BASE_SHAPES := [
	[Vector2(0, 0), Vector2(1, 0), Vector2(1, 1), Vector2(2, 1)],
	[Vector2(1, 0), Vector2(0, 1), Vector2(1, 1), Vector2(2, 1)],
	[Vector2(0, 0), Vector2(1, 0), Vector2(0, 1), Vector2(1, 1)],
	[Vector2(0, 0), Vector2(0, 1), Vector2(0, 2), Vector2(1, 2)],
	[Vector2(0, 0), Vector2(1, 0), Vector2(2, 0), Vector2(3, 0)]
]

var cells := []
var cell_shades := []
var active_cells := []
var active_shades := []
var active_origin := Vector2()
var active_color := 0
var aim_x = SPAWN_COLUMN
var score := 0
var clears := 0
var game_over := false
var next_shape_index := 0
var next_color := 0
var next_shades := []
var is_dropping := false
var drop_steps := 0
var clear_events := []
var combo_multiplier := 1
var combo_time_remaining := 0.0

func _init() -> void:
	reset()

func reset() -> void:
	cells.clear()
	cell_shades.clear()
	for _row in range(HEIGHT):
		var row := []
		var shade_row := []
		for _column in range(WIDTH):
			row.append(EMPTY)
			shade_row.append(0)
		cells.append(row)
		cell_shades.append(shade_row)
	active_cells.clear()
	active_shades.clear()
	aim_x = SPAWN_COLUMN
	score = 0
	clears = 0
	game_over = false
	is_dropping = false
	drop_steps = 0
	clear_events.clear()
	combo_multiplier = 1
	combo_time_remaining = 0.0
	_choose_next_piece()
	_spawn_piece()

func advance_piece() -> bool:
	if game_over:
		return false
	if active_cells.empty():
		_spawn_piece()
		return false
	var next_origin = active_origin + Vector2.DOWN
	if _can_place(next_origin, active_cells):
		active_origin = next_origin
		if is_dropping:
			drop_steps += 1
	else:
		_lock_piece()
		return true
	return false

func advance_sand() -> bool:
	if game_over:
		return false
	return _advance_sand()

func check_clears() -> int:
	return _clear_connected_colors()

func update_combo(delta: float) -> void:
	if combo_time_remaining <= 0.0:
		return
	combo_time_remaining = max(0.0, combo_time_remaining - delta)
	if combo_time_remaining == 0.0:
		combo_multiplier = 1

func move(direction: int) -> void:
	if game_over or active_cells.empty():
		return
	var next_origin = active_origin + Vector2(direction, 0)
	if _can_place(next_origin, active_cells):
		active_origin = next_origin
		aim_x = int(active_origin.x)

func rotate(clockwise: bool) -> void:
	if game_over or active_cells.empty():
		return
	var rotated = []
	if clockwise:
		var max_y = 0
		for cell in active_cells:
			max_y = max(max_y, int(cell.y))
		for cell in active_cells:
			rotated.append(Vector2(max_y - cell.y, cell.x))
	else:
		var max_x = 0
		for cell in active_cells:
			max_x = max(max_x, int(cell.x))
		for cell in active_cells:
			rotated.append(Vector2(cell.y, max_x - cell.x))
	var min_x = 0
	var min_y = 0
	for cell in rotated:
		min_x = min(min_x, int(cell.x))
		min_y = min(min_y, int(cell.y))
	for index in range(rotated.size()):
		rotated[index].x -= min_x
		rotated[index].y -= min_y
	if _can_place(active_origin, rotated):
		active_cells = rotated

func release_piece() -> void:
	if game_over or active_cells.empty():
		return
	is_dropping = true
	drop_steps = 0

func drop_at(column: int) -> void:
	if game_over or active_cells.empty():
		return
	var min_x = WIDTH
	var max_x = 0
	for cell in active_cells:
		min_x = min(min_x, int(cell.x))
		max_x = max(max_x, int(cell.x))
	var target_x = int(clamp(column - int(float(max_x - min_x) * 0.5), 0, WIDTH - 1 - max_x))
	var target_origin = Vector2(target_x, active_origin.y)
	if _can_place(target_origin, active_cells):
		active_origin = target_origin
		aim_x = target_x
		release_piece()

func piece_tick_interval() -> float:
	if not is_dropping:
		return 0.08
	return max(0.014, 0.045 - drop_steps * 0.003)

func get_cell(column: int, row: int) -> int:
	return cells[row][column]

func get_cell_shade(column: int, row: int) -> int:
	return cell_shades[row][column]

func get_active_world_particles() -> Array:
	var result = []
	for index in range(active_cells.size()):
		result.append(
			{
				"position": active_origin + active_cells[index],
				"shade": active_shades[index]
			}
		)
	return result

func get_next_cells() -> Array:
	return _scaled_shape(BASE_SHAPES[next_shape_index])

func get_next_shade(index: int) -> int:
	return next_shades[index]

func consume_clear_events() -> Array:
	var events = clear_events
	clear_events = []
	return events

func _spawn_piece() -> void:
	active_cells = _scaled_shape(BASE_SHAPES[next_shape_index])
	active_shades = next_shades.duplicate()
	active_color = next_color
	is_dropping = false
	drop_steps = 0
	_choose_next_piece()
	var max_x = 0
	for cell in active_cells:
		max_x = max(max_x, int(cell.x))
	active_origin = Vector2(clamp(aim_x, 0, WIDTH - 1 - max_x), 0)
	if not _can_place(active_origin, active_cells):
		active_cells.clear()
		game_over = true

func _choose_next_piece() -> void:
	next_shape_index = randi() % BASE_SHAPES.size()
	next_color = randi() % 3
	next_shades.clear()
	for _cell in _scaled_shape(BASE_SHAPES[next_shape_index]):
		next_shades.append(randi() % 6)

func _can_place(origin: Vector2, shape: Array) -> bool:
	for cell in shape:
		var position: Vector2 = origin + cell
		var column := int(position.x)
		var row := int(position.y)
		if column < 0 or column >= WIDTH or row < 0 or row >= HEIGHT:
			return false
		if cells[row][column] != EMPTY:
			return false
	return true

func _lock_piece() -> void:
	for index in range(active_cells.size()):
		var position: Vector2 = active_origin + active_cells[index]
		cells[int(position.y)][int(position.x)] = active_color
		cell_shades[int(position.y)][int(position.x)] = active_shades[index]
	active_cells.clear()
	active_shades.clear()
	_spawn_piece()

func _advance_sand() -> bool:
	var moved := false
	for row in range(HEIGHT - 2, -1, -1):
		for column in range(WIDTH):
			if cells[row][column] == EMPTY:
				continue
			if cells[row + 1][column] == EMPTY:
				_move_cell(column, row, column, row + 1)
				moved = true
				continue
			var can_move_left = column > 0 and cells[row + 1][column - 1] == EMPTY
			var can_move_right = column < WIDTH - 1 and cells[row + 1][column + 1] == EMPTY
			if not can_move_left and not can_move_right:
				continue
			var diagonal_column := column - 1
			if can_move_right and (not can_move_left or randi() % 2 == 0):
				diagonal_column = column + 1
			_move_cell(column, row, diagonal_column, row + 1)
			moved = true
	return moved

func _move_cell(from_column: int, from_row: int, to_column: int, to_row: int) -> void:
	cells[to_row][to_column] = cells[from_row][from_column]
	cell_shades[to_row][to_column] = cell_shades[from_row][from_column]
	cells[from_row][from_column] = EMPTY
	cell_shades[from_row][from_column] = 0

func _clear_connected_colors() -> int:
	var cleared_particles = 0
	var cleared_groups = 0
	var visited = []
	for _row in range(HEIGHT):
		var row = []
		for _column in range(WIDTH):
			row.append(false)
		visited.append(row)

	for row in range(HEIGHT):
		for column in range(WIDTH):
			var color = cells[row][column]
			if color == EMPTY or visited[row][column]:
				continue
			var group = _connected_group(column, row, color, visited)
			var touches_left = false
			var touches_right = false
			for position in group:
				touches_left = touches_left or int(position.x) == 0
				touches_right = touches_right or int(position.x) == WIDTH - 1
			if touches_left and touches_right:
				for position in group:
					clear_events.append(
						{
							"position": position,
							"color": color,
							"shade": cell_shades[int(position.y)][int(position.x)]
						}
					)
					cells[int(position.y)][int(position.x)] = EMPTY
					cell_shades[int(position.y)][int(position.x)] = 0
				cleared_particles += group.size()
				cleared_groups += 1
	if cleared_particles > 0:
		if combo_time_remaining > 0.0:
			combo_multiplier += 1
		else:
			combo_multiplier = 1
		combo_time_remaining = COMBO_WINDOW_SECONDS
		score += cleared_particles * 25 * combo_multiplier
		clears += cleared_groups
	return cleared_particles

func _connected_group(start_column: int, start_row: int, color: int, visited: Array) -> Array:
	var group = []
	var pending = [Vector2(start_column, start_row)]
	visited[start_row][start_column] = true
	while not pending.empty():
		var position: Vector2 = pending.pop_back()
		group.append(position)
		for y_offset in range(-1, 2):
			for x_offset in range(-1, 2):
				if x_offset == 0 and y_offset == 0:
					continue
				var column = int(position.x) + x_offset
				var row = int(position.y) + y_offset
				if column < 0 or column >= WIDTH or row < 0 or row >= HEIGHT:
					continue
				if visited[row][column] or cells[row][column] != color:
					continue
				visited[row][column] = true
				pending.append(Vector2(column, row))
	return group

func _scaled_shape(base_shape: Array) -> Array:
	var scaled = []
	for cell in base_shape:
		for y_offset in range(PARTICLE_SCALE):
			for x_offset in range(PARTICLE_SCALE):
				scaled.append(cell * PARTICLE_SCALE + Vector2(x_offset, y_offset))
	return scaled
