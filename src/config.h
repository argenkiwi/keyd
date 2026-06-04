/*
 * keyd - A key remapping daemon.
 *
 * © 2019 Raheman Vaiya (see also: LICENSE).
 */
#ifndef CONFIG_H
#define CONFIG_H

#include <limits.h>
#include "macro.h"

#define MAX_LAYER_NAME_LEN	64
#define MAX_DESCRIPTOR_ARGS	3

#define MAX_LAYERS		32
#define MAX_EXP_LEN		512


#ifndef PATH_MAX
#define PATH_MAX 1024
#endif

#define ID_EXCLUDED	1
#define ID_MOUSE	2
#define ID_KEYBOARD	4
#define ID_TRACKPAD	8
#define ID_KEY		16

enum op {
	OP_KEYSEQUENCE = 1,

	OP_ONESHOT,
	OP_ONESHOTM,
	OP_ONESHOTK,
	OP_LAYERM,
	OP_SWAP,
	OP_SWAPM,
	OP_LAYER,
	OP_LAYOUT,
	OP_CLEAR,
	OP_CLEARM,
	OP_OVERLOAD,
	OP_OVERLOAD_TIMEOUT,
	OP_OVERLOAD_TIMEOUT_TAP,
	OP_OVERLOAD_IDLE_TIMEOUT,
	OP_TOGGLE,
	OP_TOGGLEM,
	OP_REPEAT,

	OP_MACRO,
	OP_MACRO2,
	OP_COMMAND,
	OP_TIMEOUT,

/* Experimental */
	OP_SCROLL_TOGGLE_ON,
	OP_SCROLL_TOGGLE_OFF,
	OP_SCROLL_TOGGLE,
	OP_SCROLL,
};

union descriptor_arg {
	uint8_t code;
	uint8_t mods;
	int16_t idx;
	uint16_t sz;
	uint16_t timeout;
	int16_t sensitivity;
};

/*
 * Named per-variant structs for struct descriptor.
 * Each maps directly to one or more enum op values.
 * All must fit within union descriptor_arg[MAX_DESCRIPTOR_ARGS] (6 bytes).
 */
struct desc_keysequence { uint8_t code; uint8_t mods; };            /* OP_KEYSEQUENCE */
struct desc_layer        { int16_t idx; };                           /* OP_LAYER, OP_ONESHOT, OP_TOGGLE, OP_SWAP, OP_LAYOUT */
struct desc_macro        { int16_t macro_idx; };                     /* OP_MACRO, OP_CLEARM */
struct desc_command      { int16_t cmd_idx; };                       /* OP_COMMAND */
struct desc_scroll       { int16_t sensitivity; };                   /* OP_SCROLL, OP_SCROLL_TOGGLE_ON, OP_SCROLL_TOGGLE */
struct desc_layer_macro  { int16_t idx; int16_t macro_idx; };        /* OP_LAYERM, OP_SWAPM, OP_TOGGLEM, OP_ONESHOTM */
struct desc_overload     { int16_t layer_idx; int16_t action_idx; }; /* OP_OVERLOAD, OP_ONESHOTK */
struct desc_overload_to  { int16_t layer_idx; int16_t action_idx; uint16_t timeout; }; /* OP_OVERLOAD_TIMEOUT, OP_OVERLOAD_TIMEOUT_TAP */
struct desc_overload_idle { int16_t action1_idx; int16_t action2_idx; uint16_t timeout; }; /* OP_OVERLOAD_IDLE_TIMEOUT */
struct desc_timeout      { int16_t action1_idx; uint16_t timeout; int16_t action2_idx; }; /* OP_TIMEOUT */
struct desc_macro2       { uint16_t delay; uint16_t interval; int16_t macro_idx; };   /* OP_MACRO2 */

_Static_assert(sizeof(struct desc_keysequence)   <= sizeof(union descriptor_arg[MAX_DESCRIPTOR_ARGS]), "desc_keysequence too large");
_Static_assert(sizeof(struct desc_layer_macro)   <= sizeof(union descriptor_arg[MAX_DESCRIPTOR_ARGS]), "desc_layer_macro too large");
_Static_assert(sizeof(struct desc_overload)      <= sizeof(union descriptor_arg[MAX_DESCRIPTOR_ARGS]), "desc_overload too large");
_Static_assert(sizeof(struct desc_overload_to)   <= sizeof(union descriptor_arg[MAX_DESCRIPTOR_ARGS]), "desc_overload_to too large");
_Static_assert(sizeof(struct desc_overload_idle) <= sizeof(union descriptor_arg[MAX_DESCRIPTOR_ARGS]), "desc_overload_idle too large");
_Static_assert(sizeof(struct desc_timeout)       <= sizeof(union descriptor_arg[MAX_DESCRIPTOR_ARGS]), "desc_timeout too large");
_Static_assert(sizeof(struct desc_macro2)        <= sizeof(union descriptor_arg[MAX_DESCRIPTOR_ARGS]), "desc_macro2 too large");

/* Describes the intended purpose of a key (corresponds to an 'action' in user parlance). */

struct descriptor {
	enum op op;
	union {
		struct desc_keysequence  keysequence;
		struct desc_layer        layer;
		struct desc_macro        macro_op;
		struct desc_command      command;
		struct desc_scroll       scroll;
		struct desc_layer_macro  layer_macro;
		struct desc_overload     overload;
		struct desc_overload_to  overload_to;
		struct desc_overload_idle overload_idle;
		struct desc_timeout      timeout_op;
		struct desc_macro2       macro2;
		union descriptor_arg     args[MAX_DESCRIPTOR_ARGS]; /* raw access */
	};
};

struct chord {
	uint8_t keys[8];
	size_t sz;

	struct descriptor d;
};

/*
 * A layer is a map from keycodes to descriptors. It may optionally
 * contain one or more modifiers which are applied to the base layout in
 * the event that no matching descriptor is found in the keymap. For
 * consistency, modifiers are internally mapped to eponymously named
 * layers consisting of the corresponding modifier and an empty keymap.
 */

struct layer {
	char name[MAX_LAYER_NAME_LEN+1];

	enum {
		LT_NORMAL,
		LT_LAYOUT,
		LT_COMPOSITE,
	} type;

	uint8_t mods;
	struct descriptor keymap[256];

	struct chord chords[64];
	size_t nr_chords;

	/* Used for composite layers. */
	size_t nr_constituents;
	int constituents[8];
};

struct command {
	char cmd[256];
};

struct config {
	char path[PATH_MAX];
	struct layer layers[MAX_LAYERS];

	/* Auxiliary descriptors used by layer bindings. */
	struct descriptor descriptors[1024];
	struct macro macros[256];
	struct command commands[64];
	char aliases[256][32];

	uint8_t wildcard;
	struct {
		char id[64];
		uint8_t flags;
	} ids[64];


	size_t nr_ids;

	size_t nr_layers;
	size_t nr_macros;
	size_t nr_descriptors;
	size_t nr_commands;

	long macro_timeout;
	long macro_sequence_timeout;
	long macro_repeat_timeout;
	long oneshot_timeout;

	long overload_tap_timeout;

	long chord_interkey_timeout;
	long chord_hold_timeout;

	uint8_t layer_indicator;
	uint8_t disable_modifier_guard;
	char default_layout[MAX_LAYER_NAME_LEN];
};

int config_parse(struct config *config, const char *path);
int config_add_entry(struct config *config, const char *exp);
int config_get_layer_index(const struct config *config, const char *name);

int config_check_match(struct config *config, const char *id, uint8_t flags);

#endif
