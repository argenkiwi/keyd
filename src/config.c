/*
 * keyd - A key remapping daemon.
 *
 * © 2019 Raheman Vaiya (see also: LICENSE).
 */

#include <assert.h>
#include <fcntl.h>
#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <sys/types.h>
#include <unistd.h>
#include <libgen.h>

#include "keyd.h"
#include "config_parse.h"
#include "ini.h"
#include "keys.h"
#include "log.h"
#include "string.h"

#define MAX_LINES 4096
#define MAX_INCLUDES 128

struct srcmap_entry {
	const char *path;
	size_t line;
};

struct srcmap {
	char paths[MAX_INCLUDES+1][PATH_MAX];
	size_t num_paths;

	struct srcmap_entry entries[MAX_LINES];
};

static int exists_and_is_relative(const char *parent_dir, const char *path)
{
	char p1[PATH_MAX];
	char p2[PATH_MAX];

	if (!realpath(parent_dir, p1))
		return 0;

	if (!realpath(path, p2))
		return 0;

	return !strncmp(p2, p1, strlen(p1));
}

static int resolve_include_path(const char *config_path, const char *include_path, char *resolved_path)
{
	size_t len;
	size_t ret;
	char config_dir[PATH_MAX-1];

	snprintf(config_dir, sizeof config_dir, "%s", config_path);
	if (!dirname(config_dir))
		return -1;

	assert(strlen(config_dir) + strlen(include_path) + 2 < PATH_MAX);

	len = strlen(config_dir);
	strcpy(resolved_path, config_dir);
	resolved_path[len] = '/';
	strcpy(resolved_path + len + 1, include_path);

	if (exists_and_is_relative(config_dir, resolved_path))
		return 0;

	snprintf(resolved_path, PATH_MAX, DATA_DIR"/%s", include_path);
	return !exists_and_is_relative(DATA_DIR, resolved_path);
}

static void append_line(char *buf, size_t buf_sz, size_t *off, const char *line)
{
	size_t len = strlen(line);
	assert(*off + len + 2 < buf_sz);

	memcpy(buf + *off, line, len);
	buf[*off + len] = '\n';
	buf[*off + len + 1] = 0;

	*off += len + 1;
}

static const char *read_line(FILE *fh, struct parse_ctx *ctx)
{
	size_t n = 0;
	int c;

	while (1) {
		c = fgetc(fh);
		if (c == -1 || c == '\n')
			break;

		assert(n < (sizeof(ctx->line_buf)-1));
		ctx->line_buf[n++] = c;
	}

	if (n == 0 && c == -1)
		return NULL;

	ctx->line_buf[n] = 0;
	return ctx->line_buf;
}

static char *read_config_file(const char *path, struct srcmap *srcmap, struct parse_ctx *ctx)
{
	FILE *fh;
	const char *line;

	const char include_prefix[] = "include ";
	const size_t include_prefix_len = sizeof(include_prefix) - 1;

	size_t off = 0;

	size_t config_line_num = 0;
	size_t output_line_num = 0;

	if (!(fh = fopen(path, "r")))
		return NULL;

	srcmap->num_paths = 1;
	snprintf(srcmap->paths[0], sizeof srcmap->paths[0], "%s", path);

	while ((line = read_line(fh, ctx))) {
		ctx->current_line = config_line_num;
		ctx->current_file = path;

		if (!strncmp(line, include_prefix, include_prefix_len)) {
			char *include_path;

			assert(srcmap->num_paths < ARRAY_SIZE(srcmap->paths));

			include_path = srcmap->paths[srcmap->num_paths];
			if (!resolve_include_path(path, line + include_prefix_len, include_path)) {
				FILE *fh;
				size_t include_line_num = 0;

				if (!(fh = fopen(include_path, "r"))) {
					config_warn(ctx, "failed to open %s", include_path);
					continue;
				}

				srcmap->num_paths++;
				while ((line = read_line(fh, ctx))) {
					append_line(ctx->file_buf, sizeof ctx->file_buf, &off, line);

					assert(output_line_num < ARRAY_SIZE(srcmap->entries));
					srcmap->entries[output_line_num].path = include_path;
					srcmap->entries[output_line_num].line = include_line_num++;

					output_line_num++;
				}

				fclose(fh);
			} else {
				config_warn(ctx, "failed to resolve include path %s", line + include_prefix_len);
			}
		} else {
			append_line(ctx->file_buf, sizeof ctx->file_buf, &off, line);

			assert(output_line_num < ARRAY_SIZE(srcmap->entries));
			srcmap->entries[output_line_num].line = config_line_num;
			srcmap->entries[output_line_num].path = srcmap->paths[0];

			output_line_num++;
		}

		config_line_num++;
	}

	fclose(fh);
	return ctx->file_buf;
}


static struct descriptor *layer_lookup_chord(struct layer *layer, uint8_t *keys, size_t n)
{
	size_t i;

	for (i = 0; i < layer->nr_chords; i++) {
		size_t j;
		size_t nm = 0;
		struct chord *chord = &layer->chords[i];

		for (j = 0; j < n; j++) {
			size_t k;
			for (k = 0; k < chord->sz; k++)
				if (keys[j] == chord->keys[k]) {
					nm++;
					break;
				}
		}

		if (nm == n)
			return &chord->d;
	}

	return NULL;
}

/*
 * Consumes a string of the form `[<layer>.]<key> = <descriptor>` and adds the
 * mapping to the corresponding layer in the config.
 */

static int set_layer_entry(const struct config *config,
			   struct layer *layer, char *key,
			   const struct descriptor *d)
{
	size_t i;
	int found = 0;

	if (strchr(key, '+')) {
		//TODO: Handle aliases
		char *tok;
		struct descriptor *ld;
		uint8_t keys[ARRAY_SIZE(layer->chords[0].keys)];
		size_t n = 0;

		for (tok = strtok(key, "+"); tok; tok = strtok(NULL, "+")) {
			uint8_t code = config_lookup_keycode(tok);
			if (!code) {
				err("%s is not a valid key", tok);
				return -1;
			}

			if (n >= ARRAY_SIZE(keys)) {
				err("chords cannot contain more than %ld keys", n);
				return -1;
			}

			keys[n++] = code;
		}


		if ((ld = layer_lookup_chord(layer, keys, n))) {
			*ld = *d;
		} else {
			struct chord *chord;
			if (layer->nr_chords >= ARRAY_SIZE(layer->chords)) {
				err("max chords exceeded(%ld)", layer->nr_chords);
				return -1;
			}

			chord = &layer->chords[layer->nr_chords];
			memcpy(chord->keys, keys, sizeof keys);
			chord->sz = n;
			chord->d = *d;

			layer->nr_chords++;
		}
	} else {
		for (i = 0; i < 256; i++) {
			if (!strcmp(config->aliases[i], key)) {
				layer->keymap[i] = *d;
				found = 1;
			}
		}

		if (!found) {
			uint8_t code;

			if (!(code = config_lookup_keycode(key))) {
				err("%s is not a valid key or alias", key);
				return -1;
			}

			layer->keymap[code] = *d;

		}
	}

	return 0;
}

static int new_layer(char *s, const struct config *config, struct layer *layer, struct parse_ctx *ctx)
{
	uint8_t mods;
	char *name;
	char *type;

	name = strtok(s, ":");
	type = strtok(NULL, ":");

	strcpy(layer->name, name);

	layer->nr_chords = 0;

	if (strchr(name, '+')) {
		char *layername;
		int n = 0;

		layer->type = LT_COMPOSITE;
		layer->nr_constituents = 0;

		if (type) {
			err("composite layers cannot have a type.");
			return -1;
		}

		for (layername = strtok(name, "+"); layername; layername = strtok(NULL, "+")) {
			int idx = config_get_layer_index(config, layername);

			if (idx < 0) {
				err("%s is not a valid layer", layername);
				return -1;
			}

			if (n >= ARRAY_SIZE(layer->constituents)) {
				err("max composite layers (%d) exceeded", ARRAY_SIZE(layer->constituents));
				return -1;
			}

			layer->constituents[layer->nr_constituents++] = idx;
		}

	} else if (type && !strcmp(type, "layout")) {
			layer->type = LT_LAYOUT;
	} else if (type && !parse_modset(type, &mods)) {
			layer->type = LT_NORMAL;
			layer->mods = mods;
	} else {
		if (type)
			config_warn(ctx, "\"%s\" is not a valid layer type, ignoring", type);

		layer->type = LT_NORMAL;
		layer->mods = 0;
	}


	return 0;
}

/*
 * Returns:
 * 	1 if the layer exists
 * 	0 if the layer was created successfully
 * 	< 0 on error
 */
static int config_add_layer(struct config *config, const char *s, struct parse_ctx *ctx)
{
	int ret;
	char buf[MAX_LAYER_NAME_LEN+1];
	char *name;

	if (strlen(s) >= sizeof buf) {
		err("%s exceeds maximum section length (%d) (ignoring)", s, MAX_LAYER_NAME_LEN);
		return -1;
	}

	strcpy(buf, s);
	name = strtok(buf, ":");

	if (config_get_layer_index(config, name) != -1)
			return 1;

	if (config->nr_layers >= MAX_LAYERS) {
		err("max layers (%d) exceeded", MAX_LAYERS);
		return -1;
	}

	strcpy(buf, s);
	ret = new_layer(buf, config, &config->layers[config->nr_layers], ctx);

	if (ret < 0)
		return -1;

	config->nr_layers++;
	return 0;
}

static void parse_global_section(struct config *config, struct ini_section *section, struct parse_ctx *ctx)
{
	size_t i;

	for (i = 0; i < section->nr_entries;i++) {
		struct ini_entry *ent = &section->entries[i];

		if (!strcmp(ent->key, "macro_timeout"))
			config->macro_timeout = atoi(ent->val);
		else if (!strcmp(ent->key, "macro_sequence_timeout"))
			config->macro_sequence_timeout = atoi(ent->val);
		else if (!strcmp(ent->key, "disable_modifier_guard"))
			config->disable_modifier_guard = atoi(ent->val);
		else if (!strcmp(ent->key, "oneshot_timeout"))
			config->oneshot_timeout = atoi(ent->val);
		else if (!strcmp(ent->key, "chord_hold_timeout"))
			config->chord_hold_timeout = atoi(ent->val);
		else if (!strcmp(ent->key, "chord_timeout"))
			config->chord_interkey_timeout = atoi(ent->val);
		else if (!strcmp(ent->key, "default_layout"))
			snprintf(config->default_layout, sizeof config->default_layout,
				 "%s", ent->val);
		else if (!strcmp(ent->key, "macro_repeat_timeout"))
			config->macro_repeat_timeout = atoi(ent->val);
		else if (!strcmp(ent->key, "layer_indicator"))
			config->layer_indicator = atoi(ent->val);
		else if (!strcmp(ent->key, "overload_tap_timeout"))
			config->overload_tap_timeout = atoi(ent->val);
		else
			config_warn(ctx, "%s is not a valid global option", ent->key);
	}
}

static void parse_id_section(struct config *config, struct ini_section *section, struct parse_ctx *ctx)
{
	size_t i;
	for (i = 0; i < section->nr_entries; i++) {
		struct ini_entry *ent = &section->entries[i];
		const char *s = ent->key;

		if (!strcmp(s, "*")) {
			config->wildcard = 1;
		} else if (strstr(s, "m:") == s) {
			assert(config->nr_ids < ARRAY_SIZE(config->ids));
			config->ids[config->nr_ids].flags = ID_MOUSE;

			snprintf(config->ids[config->nr_ids++].id, sizeof(config->ids[0].id), "%s", s+2);
		} else if (strstr(s, "k:") == s) {
			assert(config->nr_ids < ARRAY_SIZE(config->ids));
			config->ids[config->nr_ids].flags = ID_KEYBOARD | ID_KEY;

			snprintf(config->ids[config->nr_ids++].id, sizeof(config->ids[0].id), "%s", s+2);
		} else if (strstr(s, "-") == s) {
			assert(config->nr_ids < ARRAY_SIZE(config->ids));
			config->ids[config->nr_ids].flags = ID_EXCLUDED;

			snprintf(config->ids[config->nr_ids++].id, sizeof(config->ids[0].id), "%s", s+1);
		} else if (strlen(s) < sizeof(config->ids[config->nr_ids].id)-1) {
			assert(config->nr_ids < ARRAY_SIZE(config->ids));
			config->ids[config->nr_ids].flags = ID_KEYBOARD | ID_KEY | ID_MOUSE;

			snprintf(config->ids[config->nr_ids++].id, sizeof(config->ids[0].id), "%s", s);
		} else {
			config_warn(ctx, "%s is not a valid device id", s);
		}
	}
}

static void parse_alias_section(struct config *config, struct ini_section *section, struct parse_ctx *ctx)
{
	size_t i;

	for (i = 0; i < section->nr_entries; i++) {
		uint8_t code;
		struct ini_entry *ent = &section->entries[i];
		const char *name = ent->val;

		if ((code = config_lookup_keycode(ent->key))) {
			ssize_t len = strlen(name);

			if (len >= (ssize_t)sizeof(config->aliases[0])) {
				config_warn(ctx, "%s exceeds the maximum alias length (%ld)", name, sizeof(config->aliases[0])-1);
			} else {
				uint8_t alias_code;

				if ((alias_code = config_lookup_keycode(name))) {
					struct descriptor *d = &config->layers[0].keymap[code];

					d->op = OP_KEYSEQUENCE;
					d->args[0].code = alias_code;
					d->args[1].mods = 0;
				}

				strcpy(config->aliases[code], name);
			}
		} else {
			config_warn(ctx, "failed to define alias %s, %s is not a valid keycode", name, ent->key);
		}
	}
}

// Returns 0 on success, or else the number of warnings issued.
static int do_parse(struct config *config, char *content, const struct srcmap *srcmap, struct parse_ctx *ctx)
{
	size_t i;
	struct ini *ini;

	ctx->current_file = NULL;
	ctx->current_line = 0;
	ctx->nr_warnings = 0;

	if (!(ini = ini_parse_string(content, NULL))) {
		config_warn(ctx, "Invalid config file (missing [ids] section)");
		return 1;
	}

	/* First pass: create all layers based on section headers.  */
	for (i = 0; i < ini->nr_sections; i++) {
		struct ini_section *section = &ini->sections[i];

		if (!strcmp(section->name, "ids")) {
			parse_id_section(config, section, ctx);
		} else if (!strcmp(section->name, "aliases")) {
			parse_alias_section(config, section, ctx);
		} else if (!strcmp(section->name, "global")) {
			parse_global_section(config, section, ctx);
		} else {
			if (config_add_layer(config, section->name, ctx) < 0)
				config_warn(ctx, "%s", errstr);
		}
	}

	/* Populate each layer. */
	for (i = 0; i < ini->nr_sections; i++) {
		size_t j;
		char *layername;
		struct ini_section *section = &ini->sections[i];

		if (!strcmp(section->name, "ids") ||
		    !strcmp(section->name, "aliases") ||
		    !strcmp(section->name, "global"))
			continue;

		layername = strtok(section->name, ":");

		for (j = 0; j < section->nr_entries;j++) {
			char entry[MAX_EXP_LEN];
			struct ini_entry *ent = &section->entries[j];

			if (srcmap) {
				ctx->current_file = srcmap->entries[ent->lnum - 1].path;
				ctx->current_line = srcmap->entries[ent->lnum - 1].line;
			} else {
				ctx->current_file = "";
				ctx->current_line = ent->lnum - 1;
			}

			if (!ent->val) {
				config_warn(ctx, "invalid binding");
				continue;
			}

			snprintf(entry, sizeof entry, "%s.%s = %s", layername, ent->key, ent->val);

			if (config_add_entry(config, entry) < 0)
				config_warn(ctx, "%s", errstr);
		}
	}

	return ctx->nr_warnings;
}

static void config_init(struct config *config)
{
	size_t nw;
	size_t i;
	struct parse_ctx ctx = {0};

	memset(config, 0, sizeof *config);

	char default_config[] =
	"[aliases]\n"

	"leftshift = shift\n"
	"rightshift = shift\n"

	"leftalt = alt\n"
	"rightalt = altgr\n"

	"leftmeta = meta\n"
	"rightmeta = meta\n"

	"leftcontrol = control\n"
	"rightcontrol = control\n"

	"[main:layout]\n"

	"shift = layer(shift)\n"
	"alt = layer(alt)\n"
	"altgr = layer(altgr)\n"
	"meta = layer(meta)\n"
	"control = layer(control)\n"

	"[control:C]\n"
	"[shift:S]\n"
	"[meta:M]\n"
	"[alt:A]\n"
	"[altgr:G]\n";

	nw = do_parse(config, default_config, NULL, &ctx);
	assert(nw == 0);

	/* In ms */
	config->chord_interkey_timeout = 50;
	config->chord_hold_timeout = 0;
	config->oneshot_timeout = 0;

	config->macro_timeout = 600;
	config->macro_repeat_timeout = 50;
}

/*
 * Returns:
 * 	0 on success (no warning)
 * 	n > 0 on partial success (where n is the number of issued warnings)
 * 	< 0 on complete failure
 */
int config_parse(struct config *config, const char *path)
{
	char *content;
	struct srcmap srcmap;
	struct parse_ctx ctx = {0};

	if (!(content = read_config_file(path, &srcmap, &ctx)))
		return -1;

	config_init(config);
	snprintf(config->path, sizeof(config->path), "%s", path);
	return do_parse(config, content, &srcmap, &ctx);
}

int config_check_match(struct config *config, const char *id, uint8_t flags)
{
	size_t i;

	for (i = 0; i < config->nr_ids; i++) {
		//Prefix match to allow matching <product>:<vendor> for backward compatibility.
		if (strstr(id, config->ids[i].id) == id) {
			if (config->ids[i].flags & ID_EXCLUDED) {
				return 0;
			} else if (config->ids[i].flags & flags) {
				return 2;
			}
		}
	}

	return config->wildcard ? 1 : 0;
}

int config_get_layer_index(const struct config *config, const char *name)
{
	size_t i;

	if (!name)
		return -1;

	for (i = 0; i < config->nr_layers; i++)
		if (!strcmp(config->layers[i].name, name))
			return i;

	return -1;
}

/*
 * Adds a binding of the form [<layer>.]<key> = <descriptor expression>
 * to the given config.
 */
int config_add_entry(struct config *config, const char *exp)
{
	char *keyname, *descstr, *dot, *paren, *s;
	char *layername = "main";
	struct descriptor d;
	struct layer *layer;
	int idx;
	struct parse_ctx ctx = {0};

	char buf[MAX_EXP_LEN];

	if (strlen(exp) >= MAX_EXP_LEN) {
		err("%s exceeds maximum expression length (%d)", exp, MAX_EXP_LEN);
		return -1;
	}

	strcpy(buf, exp);
	s = buf;

	dot = strchr(s, '.');
	paren = strchr(s, '(');

	if (dot && dot != s && (!paren || dot < paren)) {
		layername = s;
		*dot = 0;
		s = dot+1;
	}

	parse_kvp(s, &keyname, &descstr);
	idx = config_get_layer_index(config, layername);

	if (idx == -1) {
		err("%s is not a valid layer", layername);
		return -1;
	}

	layer = &config->layers[idx];

	if (config_parse_descriptor(descstr, &d, config, &ctx) < 0)
		return -1;

	return set_layer_entry(config, layer, keyname, &d);
}

/*
 * Parse an in-memory config string. Useful for testing without touching the
 * filesystem. Equivalent to writing content to a temp file and calling
 * config_parse(), except source-location info in warnings will be empty.
 *
 * Returns:
 *   0  on success (no warnings)
 *   n > 0  on partial success (n warnings)
 *   < 0  on complete failure
 */
int config_parse_string(struct config *config, const char *content)
{
	char buf[MAX_FILE_SIZE];
	struct parse_ctx ctx = {0};

	if (strlen(content) >= sizeof(buf)) {
		err("config string exceeds maximum size (%d)", MAX_FILE_SIZE);
		return -1;
	}

	strcpy(buf, content);
	config_init(config);
	return do_parse(config, buf, NULL, &ctx);
}

