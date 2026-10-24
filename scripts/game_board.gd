class_name GameBoard
extends Reference

const WIDTH := 40
const HEIGHT := 64
const EMPTY := -1
const PARTICLE_SCALE := 4

const BASE_SHAPES := [
	[Vector2(0, 0), Vector2(1, 0), Vector2(1, 1), Vector2(2, 1)],
	[Vector2(1, 0), Vector2(0, 1), Vector2(1, 1), Vector2(2, 1)],
	[Vector2(0, 0), Vector2(1, 0), Vector2(0, 1), Vector2(1, 1)],
	[Vector2(0, 0), Vector2(0, 1), Vector2(0, 2), Vector2(1, 2)],
	[Vector2(0, 0), Vector2(1, 0), Vector2(2, 0), Vector2(3, 0)]
]

var cells := []
var active_cells := []
var active_origin := Vector2()
var active_color := 0
var aim_x = 20
var score := 0
var clears := 0
var game_over := false
var next_shape_index := 0
var next_color := 0
var is_dropping := false
var drop_steps := 0

func _init() -> void:
	reset()

func reset() -> void:
	cells.clear()
	for _row in range(HEIGHT):
		var row := []
		for _column in range(WIDTH):
			row.append(EMPTY)
		cells.append(row)
	active_cells.clear()
	aim_x = 20
	score = 0
	clears = 0
	game_over = false
	is_dropping = false
	drop_steps = 0
	_choose_next_piece()
	_spawn_piece()

func advance_piece() -> int:
	if game_over:
		return 0
	if active_cells.empty():
		_spawn_piece()
	else:
		var next_origin = active_origin + Vector2.DOWN
		if _can_place(next_origin, active_cells):
			active_origin = next_origin
			if is_dropping:
				drop_steps += 1
		else:
			_lock_piece()
	return _clear_connected_colors()

func advance_sand() -> int:
	if game_over:
		return 0
	_advance_sand()
	return _clear_connected_colors()

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
		return 0.20
	return max(0.035, 0.11 - drop_steps * 0.008)

func get_cell(column: int, row: int) -> int:
	return cells[row][column]

func get_active_world_cells() -> Array:
	var result = []
	for cell in active_cells:
		result.append(active_origin + cell)
	return result

func get_next_cells() -> Array:
	return _scaled_shape(BASE_SHAPES[next_shape_index])

func _spawn_piece() -> void:
	active_cells = _scaled_shape(BASE_SHAPES[next_shape_index])
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
	for cell in active_cells:
		var position: Vector2 = active_origin + cell
		cells[int(position.y)][int(position.x)] = active_color
	active_cells.clear()
	_spawn_piece()

func _advance_sand() -> void:
	for row in range(HEIGHT - 2, -1, -1):
		for column in range(WIDTH):
			if cells[row][column] == EMPTY:
				continue
			var destination = Vector2(column, row + 1)
			if cells[int(destination.y)][int(destination.x)] == EMPTY:
				_move_cell(column, row, int(destination.x), int(destination.y))
				continue
			var direction = -1 if randi() % 2 == 0 else 1
			for offset in [direction, -direction]:
				var diagonal_column = column + offset
				if diagonal_column >= 0 and diagonal_column < WIDTH and cells[row + 1][diagonal_column] == EMPTY:
					_move_cell(column, row, diagonal_column, row + 1)
					break

func _move_cell(from_column: int, from_row: int, to_column: int, to_row: int) -> void:
	cells[to_row][to_column] = cells[from_row][from_column]
	cells[from_row][from_column] = EMPTY

func _clear_connected_colors() -> int:
	var cleared_particles = 0
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
					cells[int(position.y)][int(position.x)] = EMPTY
				score += group.size() * 25
				clears += 1
				cleared_particles += group.size()
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
