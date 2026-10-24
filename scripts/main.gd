extends Node2D

const GameState = preload("res://scripts/game_state.gd")
const GameBoard = preload("res://scripts/game_board.gd")

const BACKGROUND := Color("#0c0f18")
const PANEL := Color("#20283b")
const PANEL_ACTIVE := Color("#35415e")
const TEXT := Color("#f5f6fa")
const MUTED_TEXT := Color("#aab3c8")
const ACCENT := Color("#ffcb00")
const GRID_LINE := Color(0.33, 0.40, 0.57, 0.14)
const SAND_COLORS := [Color("#ff6961"), Color("#5bcf79"), Color("#5b91ff")]
const SAND_TICK_INTERVAL := 0.05

var game_state = GameState.new()
var game = GameBoard.new()
var ui := Control.new()
var labels := {}
var board_rect := Rect2()
var play_button_rect := Rect2()
var settings_button_rect := Rect2()
var action_rects := []
var game_started := false
var paused := false
var piece_tick_elapsed := 0.0
var sand_tick_elapsed := 0.0
var score_recorded := false
var shake_time := 0.0
var shake_strength := 0.0
var clear_flash := 0.0

func _ready() -> void:
	randomize()
	game_state.load_settings()
	ui.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(ui)
	assert(get_viewport().connect("size_changed", self, "_on_viewport_size_changed") == OK)
	update()

func _on_viewport_size_changed() -> void:
	update()

func _process(delta: float) -> void:
	var effect_active := shake_time > 0.0 or clear_flash > 0.0
	shake_time = max(0.0, shake_time - delta)
	clear_flash = max(0.0, clear_flash - delta * 4.0)
	if effect_active:
		update()
	if not game_started or paused or game.game_over:
		return
	piece_tick_elapsed += delta
	sand_tick_elapsed += delta
	var cleared_particles := 0
	while sand_tick_elapsed >= SAND_TICK_INTERVAL:
		sand_tick_elapsed -= SAND_TICK_INTERVAL
		cleared_particles += game.advance_sand()
	if piece_tick_elapsed >= game.piece_tick_interval():
		piece_tick_elapsed -= game.piece_tick_interval()
		cleared_particles += game.advance_piece()
	if cleared_particles > 0:
		_start_clear_effect(cleared_particles)
	_record_score_if_needed()
	update()

func _draw() -> void:
	var screen := get_viewport_rect().size
	ui.rect_size = screen
	var margin := screen.x * 0.055
	var content_width := screen.x - margin * 2.0
	var header_height := max(150.0, screen.y * 0.15)
	var controls_height := max(96.0, screen.y * 0.10)
	var board_height := min(content_width * 1.35, screen.y - header_height - controls_height - margin * 3.0)

	draw_rect(Rect2(Vector2.ZERO, screen), BACKGROUND)
	_draw_header(margin, header_height, content_width)

	board_rect = Rect2(margin, header_height + margin, content_width, board_height)
	draw_set_transform(_board_shake_offset(), 0.0, Vector2.ONE)
	_draw_board()
	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)

	_draw_controls(margin, board_rect.end.y + margin, content_width, controls_height - margin)
	if not game_state.onboarding_complete:
		_draw_onboarding(screen, margin)
	else:
		_set_onboarding_visible(false)

func _draw_header(margin: float, header_height: float, content_width: float) -> void:
	_label("title", "SANDTRIS", Rect2(margin, header_height * 0.12, 260.0, 42.0), TEXT)
	_label("score", "SCORE  %06d" % game.score, Rect2(margin, header_height * 0.48, 250.0, 26.0), ACCENT)
	_label("best", "BEST  %06d" % game_state.high_score, Rect2(margin, header_height * 0.72, 250.0, 24.0), MUTED_TEXT)

	settings_button_rect = Rect2(margin + content_width - 112.0, header_height * 0.24, 112.0, 52.0)
	_draw_button(settings_button_rect, "SETTINGS", false, "settings")
	play_button_rect = Rect2(settings_button_rect.position.x - 124.0, settings_button_rect.position.y, 112.0, 52.0)
	var play_label := "PLAY"
	if game_started and not game.game_over:
		play_label = "RESUME" if paused else "PAUSE"
	_draw_button(play_button_rect, play_label, true, "play")
	_draw_next_piece(margin + content_width - 164.0, header_height * 0.66, header_height * 0.30)

func _draw_next_piece(x: float, y: float, height: float) -> void:
	_label("next", "NEXT", Rect2(x, y, 48.0, 20.0), MUTED_TEXT)
	var preview_cells = game.get_next_cells()
	var min_cell := Vector2(1000, 1000)
	var max_cell := Vector2.ZERO
	for cell in preview_cells:
		min_cell.x = min(min_cell.x, cell.x)
		min_cell.y = min(min_cell.y, cell.y)
		max_cell.x = max(max_cell.x, cell.x)
		max_cell.y = max(max_cell.y, cell.y)
	var cell_size := min(4.0, height / (max_cell.y - min_cell.y + 1.0))
	var preview_size := (max_cell - min_cell + Vector2.ONE) * cell_size
	var origin := Vector2(x + 52.0, y + (height - preview_size.y) * 0.5)
	for cell in preview_cells:
		var relative_cell = cell - min_cell
		var rect := Rect2(origin + relative_cell * cell_size, Vector2(cell_size - 0.5, cell_size - 0.5))
		draw_rect(rect, _sand_color(game.next_color, int(cell.x), int(cell.y)))

func _draw_board() -> void:
	draw_style_box(_rounded_panel(), board_rect)
	var columns := GameBoard.WIDTH
	var rows := GameBoard.HEIGHT
	var cell_size := min(board_rect.size.x / float(columns), board_rect.size.y / float(rows))
	var grid_size := Vector2(cell_size * columns, cell_size * rows)
	var grid_origin := board_rect.position + (board_rect.size - grid_size) * 0.5

	for column in range(columns + 1):
		var x := grid_origin.x + column * cell_size
		draw_line(Vector2(x, grid_origin.y), Vector2(x, grid_origin.y + grid_size.y), GRID_LINE, 0.5)
	for row in range(rows + 1):
		var y := grid_origin.y + row * cell_size
		draw_line(Vector2(grid_origin.x, y), Vector2(grid_origin.x + grid_size.x, y), GRID_LINE, 0.5)

	for row in range(GameBoard.HEIGHT):
		for column in range(GameBoard.WIDTH):
			var color_index = game.get_cell(column, row)
			if color_index != GameBoard.EMPTY:
				_draw_sand_cell(grid_origin, cell_size, column, row, color_index)
	for position in game.get_active_world_cells():
		_draw_sand_cell(grid_origin, cell_size, int(position.x), int(position.y), game.active_color)
	if clear_flash > 0.0:
		draw_rect(board_rect, Color(1.0, 0.8, 0.3, clear_flash * 0.16))

	var status := ""
	var hint := ""
	if not game_started:
		status = "READY TO PLAY"
		hint = "Tap PLAY to begin"
	elif game.game_over:
		status = "GAME OVER"
		hint = "Tap PLAY to start again"
	elif paused:
		status = "PAUSED"
		hint = "Tap RESUME when ready"

	_label(
		"ready",
		status,
		Rect2(board_rect.position.x, board_rect.get_center().y - 28.0, board_rect.size.x, 26.0),
		TEXT,
		Label.ALIGN_CENTER
	)
	_label(
		"ready_hint",
		hint,
		Rect2(board_rect.position.x, board_rect.get_center().y + 10.0, board_rect.size.x, 24.0),
		MUTED_TEXT,
		Label.ALIGN_CENTER
	)
	labels["ready"].visible = not status.empty()
	labels["ready_hint"].visible = not hint.empty()

func _draw_sand_cell(origin: Vector2, cell_size: float, column: int, row: int, color_index: int) -> void:
	var padding := max(2.0, cell_size * 0.07)
	var rect := Rect2(
		origin + Vector2(column * cell_size + padding, row * cell_size + padding),
		Vector2(cell_size - padding * 2.0, cell_size - padding * 2.0)
	)
	draw_rect(rect, _sand_color(color_index, column, row))

func _sand_color(color_index: int, column: int, row: int) -> Color:
	var color = SAND_COLORS[color_index]
	var variation = posmod(column * 17 + row * 31 + color_index * 13, 5)
	if variation == 0:
		return color.lightened(0.12)
	if variation == 1:
		return color.darkened(0.16)
	if variation == 2:
		return color.lightened(0.05)
	return color

func _draw_controls(margin: float, top: float, content_width: float, height: float) -> void:
	action_rects.clear()
	var gap := 12.0
	var action_height := min(74.0, height)
	var left_right_width := (content_width - gap * 4.0) * 0.28
	var rotate_width := (content_width - gap * 4.0) * 0.11
	var drop_width := content_width - left_right_width * 2.0 - rotate_width * 2.0 - gap * 4.0
	var x := margin
	for definition in [
		["LEFT", left_right_width, false],
		["ROT L", rotate_width, false],
		["DROP", drop_width, true],
		["ROT R", rotate_width, false],
		["RIGHT", left_right_width, false]
	]:
		var rect := Rect2(x, top, definition[1], action_height)
		action_rects.append(rect)
		_draw_button(rect, definition[0], definition[2], "action_%d" % action_rects.size())
		x = rect.end.x + gap

func _draw_onboarding(screen: Vector2, margin: float) -> void:
	var overlay := Rect2(margin, screen.y * 0.37, screen.x - margin * 2.0, screen.y * 0.26)
	draw_style_box(_rounded_panel(), overlay)
	_label("tutorial_title", "HOW TO PLAY", Rect2(overlay.position + Vector2(24.0, 24.0), Vector2(overlay.size.x - 48.0, 28.0)), TEXT)
	_label("tutorial_one", "Move, rotate, and drop sand shapes.", Rect2(overlay.position + Vector2(24.0, 64.0), Vector2(overlay.size.x - 48.0, 24.0)), MUTED_TEXT)
	_label("tutorial_two", "Connect matching colors from wall to wall.", Rect2(overlay.position + Vector2(24.0, 94.0), Vector2(overlay.size.x - 48.0, 24.0)), MUTED_TEXT)
	_label("tutorial_continue", "Tap anywhere here to continue", Rect2(overlay.position + Vector2(24.0, 142.0), Vector2(overlay.size.x - 48.0, 24.0)), ACCENT)
	_set_onboarding_visible(true)

func _input(event: InputEvent) -> void:
	if not (event is InputEventScreenTouch and event.pressed) and not (event is InputEventMouseButton and event.pressed and event.button_index == BUTTON_LEFT):
		return

	var point = event.position
	if not game_state.onboarding_complete:
		game_state.onboarding_complete = true
		game_state.save_settings()
		update()
		return

	if play_button_rect.has_point(point):
		_handle_play_button()
		_trigger_haptic()
		update()
		return

	if settings_button_rect.has_point(point):
		game_state.haptics_enabled = not game_state.haptics_enabled
		game_state.save_settings()
		update()
		return

	for action in action_rects:
		if action.has_point(point) and game_started and not paused and not game.game_over:
			_handle_action(action_rects.find(action))
			return

	if board_rect.has_point(point) and game_started and not paused and not game.game_over:
		var column = int((point.x - board_rect.position.x) / board_rect.size.x * GameBoard.WIDTH)
		game.drop_at(clamp(column, 0, GameBoard.WIDTH - 1))
		_trigger_haptic()
		update()

func _handle_play_button() -> void:
	if not game_started or game.game_over:
		game.reset()
		game_started = true
		paused = false
		score_recorded = false
		piece_tick_elapsed = 0.0
		sand_tick_elapsed = 0.0
		return
	paused = not paused
	piece_tick_elapsed = 0.0
	sand_tick_elapsed = 0.0

func _handle_action(index: int) -> void:
	match index:
		0:
			game.move(-1)
		1:
			game.rotate(false)
		2:
			game.release_piece()
			piece_tick_elapsed = 0.0
		3:
			game.rotate(true)
		4:
			game.move(1)
	_trigger_haptic()
	_record_score_if_needed()
	update()

func _record_score_if_needed() -> void:
	if game.game_over and not score_recorded:
		score_recorded = true
		game_state.record_score(game.score)

func _start_clear_effect(cleared_particles: int) -> void:
	shake_time = 0.16
	shake_strength = min(4.0, 1.5 + cleared_particles * 0.025)
	clear_flash = 1.0
	if game_state.haptics_enabled and OS.has_feature("mobile"):
		Input.vibrate_handheld(35)

func _board_shake_offset() -> Vector2:
	if shake_time <= 0.0:
		return Vector2.ZERO
	var fade := shake_time / 0.16
	return Vector2(
		rand_range(-shake_strength, shake_strength) * fade,
		rand_range(-shake_strength, shake_strength) * fade
	)

func _trigger_haptic() -> void:
	if game_state.haptics_enabled and OS.has_feature("mobile"):
		Input.vibrate_handheld(25)

func _draw_button(rect: Rect2, label: String, emphasized: bool, key: String) -> void:
	draw_style_box(_rounded_panel(PANEL_ACTIVE if emphasized else PANEL), rect)
	_label(key, label, rect, TEXT, Label.ALIGN_CENTER)

func _label(key: String, text: String, rect: Rect2, color: Color, alignment := Label.ALIGN_LEFT) -> void:
	var label: Label
	if labels.has(key):
		label = labels[key]
	else:
		label = Label.new()
		label.mouse_filter = Control.MOUSE_FILTER_IGNORE
		label.valign = Label.VALIGN_CENTER
		labels[key] = label
		ui.add_child(label)
	label.text = text
	label.rect_position = rect.position
	label.rect_size = rect.size
	label.align = alignment
	label.add_color_override("font_color", color)
	label.show()

func _set_onboarding_visible(visible: bool) -> void:
	for key in ["tutorial_title", "tutorial_one", "tutorial_two", "tutorial_continue"]:
		if labels.has(key):
			labels[key].visible = visible

func _rounded_panel(color := PANEL) -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = color
	style.corner_radius_top_left = 14
	style.corner_radius_top_right = 14
	style.corner_radius_bottom_left = 14
	style.corner_radius_bottom_right = 14
	return style
