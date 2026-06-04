/*
 * keyd - A key remapping daemon.
 *
 * © 2019 Raheman Vaiya (see also: LICENSE).
 */
#ifndef KEYBOARD_H
#define KEYBOARD_H

#include "keyd.h"
#include "keys.h"
#include "unicode.h"
#include "config.h"
#include "device.h"

#define MAX_ACTIVE_KEYS	32
#define CACHE_SIZE	16 //Effectively nkro

struct keyboard;

struct cache_entry {
	uint8_t code;
	struct descriptor d;
	int dl;
	int layer;
};

struct key_event {
	uint8_t code;
	uint8_t pressed;
	int timestamp;
};

struct output {
	void *ctx;
	void (*send_key)(void *ctx, uint8_t code, uint8_t state);
	void (*on_layer_change)(void *ctx, const struct keyboard *kbd, const struct layer *layer, uint8_t active);
};

enum chord_status {
	CHORD_RESOLVING,
	CHORD_INACTIVE,
	CHORD_PENDING_DISAMBIGUATION,
	CHORD_PENDING_HOLD_TIMEOUT,
};

struct chord_state {
	struct key_event queue[32];
	size_t queue_sz;

	const struct chord *match;
	int match_layer;

	uint8_t start_code;
	long last_code_time;

	enum chord_status state;
};

struct timeout_state {
	uint8_t code;
	uint8_t dl;
	uint8_t spontaneous;

	long expiration;
	long activation_time;

	struct descriptor action1;
	struct descriptor action2;
};

struct overload_state {
	uint8_t code;
	uint8_t dl;
	long expiration;

	int resolve_on_interrupt;

	struct key_event queue[32];
	size_t queue_sz;

	struct descriptor action1;
	struct descriptor action2;
};

struct macro_play_state {
	struct macro *active;
	int layer;
	long timeout;
	long repeat_interval;
};

/* May correspond to more than one physical input device. */
struct keyboard {
	const struct config *original_config;
	struct config config;
	struct output output;

	/*
	 * Cache descriptors to preserve code->descriptor
	 * mappings in the event of mid-stroke layer changes.
	 */
	struct cache_entry cache[CACHE_SIZE];

	uint8_t last_pressed_output_code;
	uint8_t last_pressed_code;

	uint8_t oneshot_latch;

	uint8_t inhibit_modifier_guard;

	struct macro_play_state macro_play;
	int overload_last_layer_code;

	long oneshot_timeout;

	long overload_start_time;

	long last_simple_key_time;

	long timeouts[128];
	size_t nr_timeouts;

	struct active_chord {
		uint8_t active;
		struct chord chord;
		int layer;
	} active_chords[KEYD_CHORD_MAX-KEYD_CHORD_1+1];

	struct chord_state chord;

	struct timeout_state pending_timeout;

	struct overload_state pending_overload;

	struct {
		long activation_time;

		uint8_t active;
		uint8_t toggled;
		uint8_t oneshot_depth;
	} layer_state[MAX_LAYERS];

	struct descriptor last_repeatable_action;

	uint8_t keystate[256];

	struct {
		int x;
		int y;

		int sensitivity; /* Mouse units per scroll unit (higher == slower scrolling). */
		int active;
	} scroll;
};

struct keyboard *new_keyboard(struct config *config, const struct output *output);

long kbd_process_events(struct keyboard *kbd, const struct key_event *events, size_t n);
int kbd_eval(struct keyboard *kbd, const char *exp);
void kbd_reset(struct keyboard *kbd);

#endif
