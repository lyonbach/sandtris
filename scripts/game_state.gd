class_name GameState
extends Reference

const SAVE_PATH := "user://sandtris.cfg"

var high_score := 0
var onboarding_complete := false
var haptics_enabled := true
var reduced_motion := false

func load_settings() -> void:
	var settings := ConfigFile.new()
	if settings.load(SAVE_PATH) != OK:
		return

	high_score = int(settings.get_value("scores", "high_score", 0))
	onboarding_complete = bool(settings.get_value("settings", "onboarding_complete", false))
	haptics_enabled = bool(settings.get_value("settings", "haptics_enabled", true))
	reduced_motion = bool(settings.get_value("settings", "reduced_motion", false))

func save_settings() -> void:
	var settings := ConfigFile.new()
	settings.set_value("scores", "high_score", high_score)
	settings.set_value("settings", "onboarding_complete", onboarding_complete)
	settings.set_value("settings", "haptics_enabled", haptics_enabled)
	settings.set_value("settings", "reduced_motion", reduced_motion)
	var result := settings.save(SAVE_PATH)
	if result != OK:
		push_error("Could not save Sandtris settings: %s" % result)

func record_score(score: int) -> bool:
	if score <= high_score:
		return false
	high_score = score
	save_settings()
	return true
