extends Node2D

const GameState = preload("res://scripts/game_state.gd")
const GameBoard = preload("res://scripts/game_board.gd")
const AudioManager = preload("res://scripts/audio_manager.gd")

const BACKGROUND := Color("#0c0f18")
const PANEL := Color("#20283b")
const PANEL_ACTIVE := Color("#35415e")
const TEXT := Color("#f5f6fa")
const MUTED_TEXT := Color("#aab3c8")
const ACCENT := Color("#ffcb00")
const SAND_COLORS := [Color("#ff6961"), Color("#5bcf79"), Color("#5b91ff")]
const SAND_TICK_INTERVAL := 0.02
const CLEAR_CHECK_INTERVAL := 0.15
const SAND_TEXTURE_REFRESH_INTERVAL := 0.05
const BOARD_DOUBLE_TAP_MSEC := 240
const BOARD_DOUBLE_TAP_DISTANCE := 44.0

var game_state = GameState.new()
var game = GameBoard.new()
var audio = AudioManager.new()
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
var clear_check_elapsed := 0.0
var sand_texture_refresh_elapsed := 0.0
var score_recorded := false
var shake_time := 0.0
var shake_strength := 0.0
var clear_flash := 0.0
var clear_particles := []
var profiling_enabled := false
var profile_update_ms := 0.0
var profile_draw_ms := 0.0
var sand_image := Image.new()
var sand_texture := ImageTexture.new()
var sand_texture_dirty := true
var last_touch_timestamp := -1000
var pending_board_tap := false
var pending_board_tap_position := Vector2.ZERO
var pending_board_tap_timestamp := 0

func _ready() -> void:
	randomize()
	profiling_enabled = OS.is_debug_build()
	game_state.load_settings()
	ui.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(ui)
	add_child(audio)
	assert(get_viewport().connect("size_changed", self, "_on_viewport_size_changed") == OK)
	_refresh_sand_texture()
	update()

func _on_viewport_size_changed() -> void:
	update()

func _process(delta: float) -> void:
	_process_pending_board_tap()
	var particles_active := _advance_clear_particles(delta)
	var effect_active := shake_time > 0.0 or clear_flash > 0.0 or particles_active
	shake_time = max(0.0, shake_time - delta)
	clear_flash = max(0.0, clear_flash - delta * 4.0)
	if effect_active:
		update()
	if not game_started or paused or game.game_over:
		return
	var update_started_at := OS.get_ticks_usec()
	game.update_combo(delta)
	piece_tick_elapsed += delta
	sand_tick_elapsed += delta
	clear_check_elapsed += delta
	sand_texture_refresh_elapsed += delta
	var cleared_particles := 0
	var static_sand_changed := false
	while sand_tick_elapsed >= SAND_TICK_INTERVAL:
		sand_tick_elapsed -= SAND_TICK_INTERVAL
		static_sand_changed = game.advance_sand() or static_sand_changed
	if piece_tick_elapsed >= game.piece_tick_interval():
		piece_tick_elapsed -= game.piece_tick_interval()
		var piece_locked = game.advance_piece()
		if piece_locked:
			audio.play_sand_settle()
			_trigger_haptic(12)
		static_sand_changed = piece_locked or static_sand_changed
	if clear_check_elapsed >= CLEAR_CHECK_INTERVAL:
		clear_check_elapsed -= CLEAR_CHECK_INTERVAL
		cleared_particles = game.check_clears()
		static_sand_changed = cleared_particles > 0 or static_sand_changed
	if cleared_particles > 0:
		_start_clear_effect(game.consume_clear_events())
		audio.play_clear(game.combo_multiplier)
	if static_sand_changed:
		sand_texture_dirty = true
	if sand_texture_dirty and sand_texture_refresh_elapsed >= SAND_TEXTURE_REFRESH_INTERVAL:
		_refresh_sand_texture()
	_record_score_if_needed()
	if profiling_enabled:
		profile_update_ms = float(OS.get_ticks_usec() - update_started_at) / 1000.0
	update()

func _draw() -> void:
	var draw_started_at := OS.get_ticks_usec()
	var screen := get_viewport_rect().size
	ui.rect_size = screen
	var margin := screen.x * 0.055
	var content_width := screen.x - margin * 2.0
	var top_inset := screen.y * 0.055
	var header_height := max(150.0, screen.y * 0.15)
	var controls_height := max(128.0, screen.y * 0.12)
	var board_height := min(content_width * 1.35, screen.y - top_inset - header_height - controls_height - margin * 3.0)

	draw_rect(Rect2(Vector2.ZERO, screen), BACKGROUND)
	_draw_header(margin, top_inset, header_height, content_width)

	board_rect = Rect2(margin, top_inset + header_height + margin, content_width, board_height)
	draw_set_transform(_board_shake_offset(), 0.0, Vector2.ONE)
	_draw_board()
	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)

	_draw_controls(margin, board_rect.end.y + margin, content_width, controls_height - margin)
	if not game_state.onboarding_complete:
		_draw_onboarding(screen, margin)
	else:
		_set_onboarding_visible(false)
	if profiling_enabled:
		profile_draw_ms = float(OS.get_ticks_usec() - draw_started_at) / 1000.0

func _draw_header(margin: float, top: float, header_height: float, content_width: float) -> void:
	_label("title", "SANDTRIS", Rect2(margin, top + header_height * 0.12, 260.0, 42.0), TEXT)
	_label("score", "SCORE  %06d" % game.score, Rect2(margin, top + header_height * 0.48, 250.0, 26.0), ACCENT)
	_label("best", "BEST  %06d" % game_state.high_score, Rect2(margin, top + header_height * 0.72, 250.0, 24.0), MUTED_TEXT)
	_label("combo", "COMBO x%d" % game.combo_multiplier, Rect2(margin + 280.0, top + header_height * 0.50, 148.0, 24.0), ACCENT)
	labels["combo"].visible = game.combo_multiplier > 1
	if profiling_enabled:
		_label(
			"performance",
			"%03d FPS  |  %.2f ms SIM  |  %.2f ms DRAW" % [Engine.get_frames_per_second(), profile_update_ms, profile_draw_ms],
			Rect2(margin, top + header_height * 0.89, 350.0, 18.0),
			MUTED_TEXT
		)

	settings_button_rect = Rect2(margin + content_width - 136.0, top + header_height * 0.20, 136.0, 64.0)
	_draw_button(settings_button_rect, "SETTINGS", false, "settings")
	play_button_rect = Rect2(settings_button_rect.position.x - 148.0, settings_button_rect.position.y, 136.0, 64.0)
	var play_label := "PLAY"
	if game_started and not game.game_over:
		play_label = "RESUME" if paused else "PAUSE"
	_draw_button(play_button_rect, play_label, true, "play")
	_draw_next_piece(margin + content_width - 164.0, top + header_height * 0.66, header_height * 0.30)

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
	for index in range(preview_cells.size()):
		var cell = preview_cells[index]
		var relative_cell = cell - min_cell
		var rect := Rect2(origin + relative_cell * cell_size, Vector2(cell_size - 0.5, cell_size - 0.5))
		draw_rect(rect, _sand_color(game.next_color, game.get_next_shade(index)))

func _draw_board() -> void:
	draw_style_box(_rounded_panel(), board_rect)
	var columns := GameBoard.WIDTH
	var rows := GameBoard.HEIGHT
	var cell_size := min(board_rect.size.x / float(columns), board_rect.size.y / float(rows))
	var grid_size := Vector2(cell_size * columns, cell_size * rows)
	var grid_origin := board_rect.position + (board_rect.size - grid_size) * 0.5

	draw_texture_rect(sand_texture, Rect2(grid_origin, grid_size), false)
	for particle in game.get_active_world_particles():
		var position: Vector2 = particle["position"]
		_draw_sand_cell(grid_origin, cell_size, int(position.x), int(position.y), game.active_color, particle["shade"])
	_draw_clear_particles(grid_origin, cell_size)
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

func _draw_sand_cell(origin: Vector2, cell_size: float, column: int, row: int, color_index: int, shade: int) -> void:
	var rect := Rect2(
		origin + Vector2(column * cell_size, row * cell_size),
		Vector2(cell_size, cell_size)
	)
	draw_rect(rect, _sand_color(color_index, shade))

func _refresh_sand_texture() -> void:
	sand_image.create(GameBoard.WIDTH, GameBoard.HEIGHT, false, Image.FORMAT_RGBA8)
	sand_image.lock()
	for row in range(GameBoard.HEIGHT):
		for column in range(GameBoard.WIDTH):
			var color_index = game.get_cell(column, row)
			var color := Color(0.0, 0.0, 0.0, 0.0)
			if color_index != GameBoard.EMPTY:
				color = _sand_color(color_index, game.get_cell_shade(column, row))
			sand_image.set_pixel(column, row, color)
	sand_image.unlock()
	if sand_texture.get_width() == 0:
		sand_texture.create_from_image(sand_image, 0)
	else:
		sand_texture.set_data(sand_image)
	sand_texture_dirty = false
	sand_texture_refresh_elapsed = 0.0

func _sand_color(color_index: int, shade: int) -> Color:
	var color = SAND_COLORS[color_index]
	if shade == 0:
		return color.lightened(0.12)
	if shade == 1:
		return color.darkened(0.16)
	if shade == 2:
		return color.lightened(0.05)
	return color

func _draw_controls(margin: float, top: float, content_width: float, height: float) -> void:
	action_rects.clear()
	var gap := 16.0
	var action_height := min(104.0, height)
	var action_width := (content_width - gap * 2.0) / 3.0
	var x := margin
	for definition in [
		["LEFT", 0, false],
		["DROP", 2, true],
		["RIGHT", 4, false]
	]:
		var rect := Rect2(x, top, action_width, action_height)
		action_rects.append({"rect": rect, "index": definition[1]})
		_draw_button(rect, definition[0], definition[2], "action_%d" % action_rects.size())
		x = rect.end.x + gap

func _draw_onboarding(screen: Vector2, margin: float) -> void:
	var overlay := Rect2(margin, screen.y * 0.37, screen.x - margin * 2.0, screen.y * 0.26)
	draw_style_box(_rounded_panel(), overlay)
	_label("tutorial_title", "HOW TO PLAY", Rect2(overlay.position + Vector2(24.0, 24.0), Vector2(overlay.size.x - 48.0, 28.0)), TEXT)
	_label("tutorial_one", "Use buttons to move and drop sand shapes.", Rect2(overlay.position + Vector2(24.0, 64.0), Vector2(overlay.size.x - 48.0, 24.0)), MUTED_TEXT)
	_label("tutorial_two", "Connect matching colors from wall to wall.", Rect2(overlay.position + Vector2(24.0, 94.0), Vector2(overlay.size.x - 48.0, 24.0)), MUTED_TEXT)
	_label("tutorial_continue", "Tap board: rotate. Double-tap: drop.", Rect2(overlay.position + Vector2(24.0, 142.0), Vector2(overlay.size.x - 48.0, 24.0)), ACCENT)
	_set_onboarding_visible(true)

func _input(event: InputEvent) -> void:
	var is_touch = event is InputEventScreenTouch and event.pressed
	var is_mouse_click = event is InputEventMouseButton and event.pressed and event.button_index == BUTTON_LEFT
	if not is_touch and not is_mouse_click:
		return

	var timestamp := OS.get_ticks_msec()
	if is_mouse_click and timestamp - last_touch_timestamp < 100:
		return
	if is_touch:
		last_touch_timestamp = timestamp
	var point = event.position
	audio.activate()
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
		audio.play_button()
		update()
		return

	for action in action_rects:
		if action["rect"].has_point(point) and game_started and not paused and not game.game_over:
			_handle_action(action["index"])
			return

	if board_rect.has_point(point) and game_started and not paused and not game.game_over:
		_handle_board_tap(point, timestamp)

func _process_pending_board_tap() -> void:
	if not pending_board_tap or OS.get_ticks_msec() - pending_board_tap_timestamp < BOARD_DOUBLE_TAP_MSEC:
		return
	pending_board_tap = false
	if not game_started or paused or game.game_over:
		return
	game.rotate(true)
	audio.play_button()
	_trigger_haptic()
	update()

func _handle_board_tap(point: Vector2, timestamp: int) -> void:
	if pending_board_tap and timestamp - pending_board_tap_timestamp <= BOARD_DOUBLE_TAP_MSEC and point.distance_to(pending_board_tap_position) <= BOARD_DOUBLE_TAP_DISTANCE:
		pending_board_tap = false
		var column = int((point.x - board_rect.position.x) / board_rect.size.x * GameBoard.WIDTH)
		game.drop_at(clamp(column, 0, GameBoard.WIDTH - 1))
		audio.play_piece_release()
		_trigger_haptic()
		update()
		return
	pending_board_tap = true
	pending_board_tap_position = point
	pending_board_tap_timestamp = timestamp

func _handle_play_button() -> void:
	if not game_started or game.game_over:
		game.reset()
		game_started = true
		paused = false
		score_recorded = false
		piece_tick_elapsed = 0.0
		sand_tick_elapsed = 0.0
		clear_check_elapsed = 0.0
		sand_texture_dirty = true
		_refresh_sand_texture()
		audio.play_button()
		return
	paused = not paused
	piece_tick_elapsed = 0.0
	sand_tick_elapsed = 0.0
	audio.play_button()

func _handle_action(index: int) -> void:
	match index:
		0:
			game.move(-1)
		1:
			game.rotate(false)
		2:
			game.release_piece()
			piece_tick_elapsed = 0.0
			audio.play_piece_release()
		3:
			game.rotate(true)
		4:
			game.move(1)
	if index != 2:
		audio.play_button()
	_trigger_haptic()
	_record_score_if_needed()
	update()

func _record_score_if_needed() -> void:
	if game.game_over and not score_recorded:
		score_recorded = true
		game_state.record_score(game.score)
		audio.play_game_over()
		_trigger_haptic(60)

func _start_clear_effect(events: Array) -> void:
	shake_time = 0.16
	shake_strength = min(4.0, 1.5 + events.size() * 0.025)
	clear_flash = 1.0
	var sample_stride := max(1, int(ceil(float(events.size()) / 96.0)))
	var pattern := randi() % 6
	for index in range(0, events.size(), sample_stride):
		var event = events[index]
		clear_particles.append(
			{
				"position": event["position"] + Vector2(0.5, 0.5),
				"color": _sand_color(event["color"], event["shade"]),
				"offset": Vector2.ZERO,
				"velocity": Vector2(rand_range(-2.0, 2.0), rand_range(-3.4, -1.2)),
				"delay": _clear_wave_delay(event["position"], pattern),
				"life": 0.32 + randf() * 0.16
			}
		)
	if game_state.haptics_enabled and OS.has_feature("mobile"):
		_trigger_haptic(35)

func _advance_clear_particles(delta: float) -> bool:
	for index in range(clear_particles.size() - 1, -1, -1):
		var particle = clear_particles[index]
		if particle["delay"] > 0.0:
			particle["delay"] -= delta
			continue
		particle["life"] -= delta
		if particle["life"] <= 0.0:
			clear_particles.remove(index)
			continue
		var velocity = particle["velocity"]
		velocity.y += 7.5 * delta
		particle["velocity"] = velocity
		var offset = particle["offset"]
		particle["offset"] = offset + velocity * delta
	return not clear_particles.empty()

func _draw_clear_particles(origin: Vector2, cell_size: float) -> void:
	for particle in clear_particles:
		if particle["delay"] > 0.0:
			continue
		var color = particle["color"]
		color.a = min(1.0, particle["life"] * 3.0)
		var center = origin + (particle["position"] + particle["offset"]) * cell_size
		draw_circle(center, max(1.0, cell_size * 0.22), color)

func _clear_wave_delay(position: Vector2, pattern: int) -> float:
	var horizontal := position.x / float(GameBoard.WIDTH - 1)
	var vertical := position.y / float(GameBoard.HEIGHT - 1)
	var distance := 0.0
	match pattern:
		0:
			distance = min(horizontal, 1.0 - horizontal) * 2.0
		1:
			distance = abs(horizontal - 0.5) * 2.0
		2:
			distance = horizontal
		3:
			distance = 1.0 - horizontal
		4:
			distance = vertical
		_:
			distance = 1.0 - vertical
	return distance * 0.32

func _board_shake_offset() -> Vector2:
	if shake_time <= 0.0:
		return Vector2.ZERO
	var fade := shake_time / 0.16
	return Vector2(
		rand_range(-shake_strength, shake_strength) * fade,
		rand_range(-shake_strength, shake_strength) * fade
	)

func _trigger_haptic(duration := 25) -> void:
	if game_state.haptics_enabled and OS.has_feature("mobile"):
		Input.vibrate_handheld(duration)

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
