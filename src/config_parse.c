/*
 * keyd - A key remapping daemon.
 *
 * © 2019 Raheman Vaiya (see also: LICENSE).
 *
 * Syntax-layer config parsing: transforms descriptor/macro/command strings
 * into their corresponding C structs. This translation unit has no dependency
 * on file I/O or INI-level config construction — it only needs a struct config
 * for layer-index lookups and storage allocation.
 */

#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>

#include "keyd.h"
#include "config_parse.h"

/* Action descriptor table — used only by config_parse_descriptor. */
static struct {
	const char *name;
	const char *preferred_name;
	uint8_t op;
	enum {
		ARG_EMPTY,

		ARG_MACRO,
		ARG_LAYER,
		ARG_LAYOUT,
		ARG_TIMEOUT,
		ARG_SENSITIVITY,
		ARG_KEYSEQUENCE_DESCRIPTOR,
		ARG_DESCRIPTOR,
	} args[MAX_DESCRIPTOR_ARGS];
} actions[] =  {
	{ "swap", 	NULL,	OP_SWAP,	{ ARG_LAYER } },
	{ "clear", 	NULL,	OP_CLEAR,	{} },
	{ "oneshot", 	NULL,	OP_ONESHOT,	{ ARG_LAYER } },
	{ "toggle", 	NULL,	OP_TOGGLE,	{ ARG_LAYER } },

	{ "clearm", 	NULL,	OP_CLEARM,	{ ARG_MACRO } },
	{ "swapm", 	NULL,	OP_SWAPM,	{ ARG_LAYER, ARG_MACRO } },
	{ "togglem", 	NULL,	OP_TOGGLEM,	{ ARG_LAYER, ARG_MACRO } },
	{ "layerm", 	NULL,	OP_LAYERM,	{ ARG_LAYER, ARG_MACRO } },
	{ "oneshotm", 	NULL,	OP_ONESHOTM,	{ ARG_LAYER, ARG_MACRO } },
	{ "oneshotk", 	NULL,	OP_ONESHOTK,	{ ARG_LAYER, ARG_KEYSEQUENCE_DESCRIPTOR } },

	{ "layer", 	NULL,	OP_LAYER,	{ ARG_LAYER } },

	{ "overload", 	NULL,	OP_OVERLOAD,			{ ARG_LAYER, ARG_DESCRIPTOR } },
	{ "overloadt", 	NULL,	OP_OVERLOAD_TIMEOUT,		{ ARG_LAYER, ARG_DESCRIPTOR, ARG_TIMEOUT } },
	{ "overloadt2", NULL,	OP_OVERLOAD_TIMEOUT_TAP,	{ ARG_LAYER, ARG_DESCRIPTOR, ARG_TIMEOUT } },

	{ "overloadi",	NULL,	OP_OVERLOAD_IDLE_TIMEOUT, { ARG_DESCRIPTOR, ARG_DESCRIPTOR, ARG_TIMEOUT } },
	{ "timeout", 	NULL,	OP_TIMEOUT,	{ ARG_DESCRIPTOR, ARG_TIMEOUT, ARG_DESCRIPTOR } },

	{ "macro2", 	NULL,	OP_MACRO2,	{ ARG_TIMEOUT, ARG_TIMEOUT, ARG_MACRO } },
	{ "setlayout", 	NULL,	OP_LAYOUT,	{ ARG_LAYOUT } },

	{ "repeat", 	NULL,	OP_REPEAT,	{} },

	/* Experimental */
	{ "scrollon", 	NULL,	OP_SCROLL_TOGGLE_ON,		{ARG_SENSITIVITY} },
	{ "scrolloff", 	NULL,	OP_SCROLL_TOGGLE_OFF,		{} },
	{ "scrollt", 	NULL,	OP_SCROLL_TOGGLE,		{ARG_SENSITIVITY} },
	{ "scroll", 	NULL,	OP_SCROLL,			{ARG_SENSITIVITY} },

	/* TODO: deprecate */
	{ "overload2", 	"overloadt",	OP_OVERLOAD_TIMEOUT,		{ ARG_LAYER, ARG_DESCRIPTOR, ARG_TIMEOUT } },
	{ "overload3", 	"overloadt2",	OP_OVERLOAD_TIMEOUT_TAP,	{ ARG_LAYER, ARG_DESCRIPTOR, ARG_TIMEOUT } },
	{ "toggle2", 	"togglem",	OP_TOGGLEM,			{ ARG_LAYER, ARG_MACRO } },
	{ "swap2", 	"swapm",	OP_SWAPM,			{ ARG_LAYER, ARG_MACRO } },
};

void config_warn(struct parse_ctx *ctx, const char *fmt, ...)
{
	va_list ap;
	char buf[1024];

	if (ctx->current_file)
		snprintf(buf, sizeof buf, "\ty{WARNING:} b{%s}:r{%zd}: %s\n", ctx->current_file, ctx->current_line + 1, fmt);
	else
		snprintf(buf, sizeof buf, "\ty{WARNING:} %s\n", fmt);

	ctx->nr_warnings++;
	va_start(ap, fmt);
	_vkeyd_log(buf, ap);
	va_end(ap);
}

uint8_t config_lookup_keycode(const char *name)
{
	size_t i;

	for (i = 0; i < 256; i++) {
		const struct keycode_table_ent *ent = &keycode_table[i];

		if (ent->name &&
		    (!strcmp(ent->name, name) ||
		     (ent->alt_name && !strcmp(ent->alt_name, name)))) {
			return i;
		}
	}

	return 0;
}

/* Parses a function call of the form name(arg1, arg2, ...) into its parts.
 * Modifies s in place. Returns 0 on success, -1 if s is not a function call. */
static int parse_fn(char *s,
		    char **name,
		    char *args[5],
		    size_t *nargs)
{
	char *c, *arg;

	c = s;
	while (*c && *c != '(')
		c++;

	if (!*c)
		return -1;

	*name = s;
	*c++ = 0;

	while (*c == ' ')
		c++;

	*nargs = 0;
	arg = c;
	while (1) {
		int plvl = 0;

		while (*c) {
			switch (*c) {
			case '\\':
				if (*(c+1)) {
					c+=2;
					continue;
				}
				break;
			case '(':
				plvl++;
				break;
			case ')':
				plvl--;

				if (plvl == -1)
					goto exit;
				break;
			case ',':
				if (plvl == 0)
					goto exit;
				break;
			}

			c++;
		}
exit:

		if (!*c)
			return -1;

		if (arg != c) {
			assert(*nargs < 5);
			args[(*nargs)++] = arg;
		}

		if (*c == ')') {
			*c = 0;
			return 0;
		}

		*c++ = 0;
		while (*c == ' ')
			c++;
		arg = c;
	}
}

int config_parse_macro_expression(const char *s, struct macro *macro)
{
	uint8_t code, mods;

	#define ADD_ENTRY(t, d) do { \
		if (macro->sz >= ARRAY_SIZE(macro->entries)) { \
			err("maximum macro size (%d) exceeded", ARRAY_SIZE(macro->entries)); \
			return 1; \
		} \
		macro->entries[macro->sz].type = t; \
		macro->entries[macro->sz].data = d; \
		macro->sz++; \
	} while(0)

	size_t len = strlen(s);

	char buf[1024];
	char *ptr = buf;

	if (len >= sizeof(buf)) {
		err("macro size exceeded maximum size (%ld)\n", sizeof(buf));
		return -1;
	}

	strcpy(buf, s);

	if (!strncmp(ptr, "macro(", 6) && ptr[len-1] == ')') {
		ptr[len-1] = 0;
		ptr += 6;
		str_escape(ptr);
	} else if (parse_key_sequence(ptr, &code, &mods) && utf8_strlen(ptr) != 1) {
		err("Invalid macro");
		return -1;
	}

	return macro_parse(ptr, macro) == 0 ? 0 : 1;
}

int config_parse_command(const char *s, struct command *command)
{
	int len = strlen(s);

	if (len == 0 || strstr(s, "command(") != s || s[len-1] != ')')
		return -1;

	if (len > (int)sizeof(command->cmd)) {
		err("max command length (%ld) exceeded\n", sizeof(command->cmd));
		return 1;
	}

	strcpy(command->cmd, s+8);
	command->cmd[len-9] = 0;
	str_escape(command->cmd);

	return 0;
}

int config_parse_descriptor(char *s,
			    struct descriptor *d,
			    struct config *config,
			    struct parse_ctx *ctx)
{
	char *fn = NULL;
	char *args[5];
	size_t nargs = 0;
	uint8_t code, mods;
	int ret;
	struct macro macro;
	struct command cmd;

	if (!s || !s[0]) {
		d->op = 0;
		return 0;
	}

	if (!parse_key_sequence(s, &code, &mods)) {
		size_t i;
		const char *layer = NULL;

		switch (code) {
			case KEYD_LEFTSHIFT:   layer = "shift"; break;
			case KEYD_LEFTCTRL:    layer = "control"; break;
			case KEYD_LEFTMETA:    layer = "meta"; break;
			case KEYD_LEFTALT:     layer = "alt"; break;
			case KEYD_RIGHTALT:    layer = "altgr"; break;
		}

		if (layer) {
			config_warn(ctx, "You should use b{layer(%s)} instead of assigning to b{%s} directly.", layer, KEY_NAME(code));
			d->op = OP_LAYER;
			d->layer.idx = config_get_layer_index(config, layer);

			assert(d->layer.idx != -1);

			return 0;
		}

		d->op = OP_KEYSEQUENCE;
		d->keysequence.code = code;
		d->keysequence.mods = mods;

		return 0;
	} else if ((ret=config_parse_command(s, &cmd)) >= 0) {
		if (ret) {
			return -1;
		}

		if (config->nr_commands >= ARRAY_SIZE(config->commands)) {
			err("max commands (%d), exceeded", ARRAY_SIZE(config->commands));
			return -1;
		}


		d->op = OP_COMMAND;
		d->command.cmd_idx = config->nr_commands;

		config->commands[config->nr_commands++] = cmd;

		return 0;
	} else if ((ret=config_parse_macro_expression(s, &macro)) >= 0) {
		if (ret)
			return -1;

		if (config->nr_macros >= ARRAY_SIZE(config->macros)) {
			err("max macros (%d), exceeded", ARRAY_SIZE(config->macros));
			return -1;
		}

		d->op = OP_MACRO;
		d->macro_op.macro_idx = config->nr_macros;

		config->macros[config->nr_macros++] = macro;

		return 0;
	} else if (!parse_fn(s, &fn, args, &nargs)) {
		int i;
		char buf[1024];

		if (!strcmp(fn, "lettermod")) {
			if (nargs != 4) {
				err("%s requires 4 arguments", fn);
				return -1;
			}

			snprintf(buf, sizeof buf,
				"overloadi(%s, overloadt2(%s, %s, %s), %s)",
				args[1], args[0], args[1], args[3], args[2]);

			if (parse_fn(buf, &fn, args, &nargs)) {
				err("failed to parse %s", buf);
				return -1;
			}
		}

		if (!strcmp(fn, "oneshot") && nargs >= 2) {
			size_t k;

			if (nargs > MAX_DESCRIPTOR_ARGS) {
				err("oneshot supports at most %d layers", MAX_DESCRIPTOR_ARGS);
				return -1;
			}

			d->op = OP_ONESHOT_MULTI;

			for (k = 0; k < nargs; k++) {
				if (!strcmp(args[k], "main")) {
					err("the main layer cannot be toggled");
					return -1;
				}

				d->layer_multi.idx[k] = config_get_layer_index(config, args[k]);
				if (d->layer_multi.idx[k] == -1 ||
				    config->layers[(int)d->layer_multi.idx[k]].type == LT_LAYOUT) {
					err("%s is not a valid layer", args[k]);
					return -1;
				}
			}

			if (nargs < MAX_DESCRIPTOR_ARGS)
				d->layer_multi.idx[nargs] = -1;

			return 0;
		}

		for (i = 0; i < ARRAY_SIZE(actions); i++) {
			if (!strcmp(actions[i].name, fn)) {
				int j;

				if (actions[i].preferred_name)
					config_warn(ctx, "%s is deprecated (renamed to %s).", actions[i].name, actions[i].preferred_name);

				d->op = actions[i].op;

				for (j = 0; j < MAX_DESCRIPTOR_ARGS; j++) {
					if (!actions[i].args[j])
						break;
				}

				if ((int)nargs != j) {
					err("%s requires %d %s", actions[i].name, j, j == 1 ? "argument" : "arguments");
					return -1;
				}

				while (j--) {
					int type = actions[i].args[j];
					union descriptor_arg *arg = &d->args[j];
					char *argstr = args[j];
					struct descriptor desc;

					switch (type) {
					case ARG_LAYER:
						if (!strcmp(argstr, "main")) {
							err("the main layer cannot be toggled");
							return -1;
						}

						arg->idx = config_get_layer_index(config, argstr);
						if (arg->idx == -1 || config->layers[arg->idx].type == LT_LAYOUT) {
							err("%s is not a valid layer", argstr);
							return -1;
						}

						break;
					case ARG_LAYOUT:
						arg->idx = config_get_layer_index(config, argstr);
						if (arg->idx == -1 ||
							(arg->idx != 0 && //Treat main as a valid layout
							 config->layers[arg->idx].type != LT_LAYOUT)) {
							err("%s is not a valid layout", argstr);
							return -1;
						}

						break;
					case ARG_KEYSEQUENCE_DESCRIPTOR:
					case ARG_DESCRIPTOR:
						if (config_parse_descriptor(argstr, &desc, config, ctx))
							return -1;

						if (config->nr_descriptors >= ARRAY_SIZE(config->descriptors)) {
							err("maximum descriptors exceeded");
							return -1;
						}

						if (type == ARG_KEYSEQUENCE_DESCRIPTOR && desc.op != OP_KEYSEQUENCE) {
							err("%s is not a valid keysequence", argstr);
							return -1;
						}

						config->descriptors[config->nr_descriptors] = desc;
						arg->idx = config->nr_descriptors++;
						break;
					case ARG_SENSITIVITY:
						arg->sensitivity = atoi(argstr);
						break;
					case ARG_TIMEOUT:
						arg->timeout = atoi(argstr);
						break;
					case ARG_MACRO:
						if (config->nr_macros >= ARRAY_SIZE(config->macros)) {
							err("max macros (%d), exceeded", ARRAY_SIZE(config->macros));
							return -1;
						}

						if (config_parse_macro_expression(argstr, &config->macros[config->nr_macros])) {
							return -1;
						}

						arg->idx = config->nr_macros;
						config->nr_macros++;

						break;
					default:
						assert(0);
						break;
					}
				}

				return 0;
			}
		}
	}

	err("invalid key or action");
	return -1;
}
