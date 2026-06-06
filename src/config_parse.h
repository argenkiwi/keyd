/*
 * keyd - A key remapping daemon.
 *
 * © 2019 Raheman Vaiya (see also: LICENSE).
 */
#ifndef CONFIG_PARSE_H
#define CONFIG_PARSE_H

#include <stdarg.h>
#include <stddef.h>
#include "config.h"
#include "macro.h"

#define MAX_FILE_SIZE 65536

/*
 * Parse context threaded through config reading and descriptor parsing.
 * Tracks source location for warnings and buffers file content during
 * config_parse(). May be zero-initialised for out-of-file callers such
 * as config_add_entry() and unit tests.
 */
struct parse_ctx {
	size_t current_line;
	const char *current_file;
	size_t nr_warnings;
	char line_buf[4096];
	char file_buf[MAX_FILE_SIZE];
};

/* Emit a source-attributed warning and increment ctx->nr_warnings. */
void config_warn(struct parse_ctx *ctx, const char *fmt, ...);

/*
 * Resolve a key name to its evdev code.
 * Returns 0 if the name is unknown.
 */
uint8_t config_lookup_keycode(const char *name);

/*
 * Parse a descriptor expression string into *d.
 * config is used for layer-index lookup only; its macro/descriptor arrays
 * are mutated as side-effects when storage is needed.
 *
 * Returns:
 *   0  on success
 *  -1  on error (errstr is set)
 */
int config_parse_descriptor(char *s,
			    struct descriptor *d,
			    struct config *config,
			    struct parse_ctx *ctx);

/*
 * Parse a macro expression string into *macro.
 *
 * Returns:
 *   0  on success
 *  -1  on invalid macro expression
 *  >0  on other errors
 */
int config_parse_macro_expression(const char *s, struct macro *macro);

/*
 * Parse a command(…) expression string into *command.
 *
 * Returns:
 *   0  on success
 *  -1  if s is not a command expression
 *  >0  on other errors
 */
int config_parse_command(const char *s, struct command *command);

#endif
