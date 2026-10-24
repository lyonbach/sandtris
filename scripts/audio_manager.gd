class_name AudioManager
extends Node

const MUSIC_TRACKS := [
	"res://assets/music/sandtris_a.ogg",
	"res://assets/music/sandtris_b.ogg",
	"res://assets/music/sandtris_c.ogg",
	"res://assets/music/sandtris_d.ogg"
]
const SAMPLE_RATE := 22050.0
const MAX_EFFECTS := 8

var effect_player := AudioStreamPlayer.new()
var music_player := AudioStreamPlayer.new()
var generator := AudioStreamGenerator.new()
var playback: AudioStreamGeneratorPlayback
var effects := []
var activated := false

func _ready() -> void:
	generator.mix_rate = int(SAMPLE_RATE)
	generator.buffer_length = 0.05
	effect_player.stream = generator
	effect_player.volume_db = -8.0
	music_player.volume_db = -17.0
	add_child(effect_player)
	add_child(music_player)

func activate() -> void:
	if activated:
		return
	activated = true
	effect_player.play()
	playback = effect_player.get_stream_playback()
	var music = _load_music(MUSIC_TRACKS[randi() % MUSIC_TRACKS.size()])
	if music:
		music_player.stream = music
		music_player.play()

func play_piece_release() -> void:
	_queue_effect("release", 0.16, 420.0, 210.0, 0.18)

func play_sand_settle() -> void:
	_queue_effect("settle", 0.10, 150.0, 80.0, 0.08)

func play_clear(combo_multiplier: int) -> void:
	var frequency := 540.0 + min(combo_multiplier, 6) * 80.0
	_queue_effect("clear", 0.30, frequency, frequency * 1.7, 0.24)

func play_game_over() -> void:
	_queue_effect("game_over", 0.65, 260.0, 60.0, 0.22)

func play_button() -> void:
	_queue_effect("button", 0.07, 620.0, 760.0, 0.10)

func _process(_delta: float) -> void:
	if not activated:
		return
	if effects.empty():
		return
	if playback == null:
		playback = effect_player.get_stream_playback()
	if playback == null:
		return
	for _frame in range(playback.get_frames_available()):
		var sample := _next_sample()
		assert(playback.push_frame(Vector2(sample, sample)))

func _next_sample() -> float:
	var sample := 0.0
	for index in range(effects.size() - 1, -1, -1):
		var effect = effects[index]
		effect["age"] += 1.0 / SAMPLE_RATE
		if effect["age"] >= effect["duration"]:
			effects.remove(index)
			continue
		var progress = effect["age"] / effect["duration"]
		var frequency = lerp(effect["start_frequency"], effect["end_frequency"], progress)
		effect["phase"] += TAU * frequency / SAMPLE_RATE
		var envelope = 1.0 - progress
		if effect["kind"] == "settle":
			sample += (randf() * 2.0 - 1.0) * envelope * effect["amplitude"]
		elif effect["kind"] == "clear":
			sample += (sin(effect["phase"]) + sin(effect["phase"] * 2.0) * 0.35) * envelope * effect["amplitude"]
		elif effect["kind"] == "button":
			sample += sign(sin(effect["phase"])) * envelope * effect["amplitude"]
		else:
			sample += sin(effect["phase"]) * envelope * effect["amplitude"]
	return clamp(sample, -0.8, 0.8)

func _queue_effect(kind: String, duration: float, start_frequency: float, end_frequency: float, amplitude: float) -> void:
	if not activated:
		return
	if effects.size() >= MAX_EFFECTS:
		effects.remove(0)
	effects.append(
		{
			"kind": kind,
			"duration": duration,
			"start_frequency": start_frequency,
			"end_frequency": end_frequency,
			"amplitude": amplitude,
			"age": 0.0,
			"phase": 0.0
		}
	)

func _load_music(path: String) -> AudioStreamOGGVorbis:
	var file := File.new()
	if file.open(path, File.READ) != OK:
		push_error("Could not open music track: %s" % path)
		return null
	var stream := AudioStreamOGGVorbis.new()
	stream.data = file.get_buffer(file.get_len())
	stream.loop = true
	return stream
