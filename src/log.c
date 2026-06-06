/*
 * keyd - A key remapping daemon.
 *
 * © 2019 Raheman Vaiya (see also: LICENSE).
 */
#include "log.h"
#include <time.h>
#include <pthread.h>

char errstr[8192];

static pthread_mutex_t mtx = PTHREAD_MUTEX_INITIALIZER;

int log_level = 0;
int suppress_colours = 0;
int log_format = LOG_FORMAT_TEXT;

static const char *colorize(const char *s)
{
	int i;

	static char buf[1024];
	size_t n  = 0;
	int inside_escape = 0;

	for (i = 0; s[i] != 0 && n < sizeof(buf); i++) {
		if (s[i+1] == '{') {
			int escape_num = 0;

			switch (s[i]) {
				case 'r': escape_num = 1; break;
				case 'g': escape_num = 2; break;
				case 'y': escape_num = 3; break;
				case 'b': escape_num = 4; break;
				case 'm': escape_num = 5; break;
				case 'c': escape_num = 6; break;
				case 'w': escape_num = 7; break;
				default: break;
			}

			if (escape_num) {
				if (!suppress_colours && (sizeof(buf)-n > 5)) {
					buf[n++] = '\033';
					buf[n++] = '[';
					buf[n++] = '3';
					buf[n++] = '0' + escape_num;
					buf[n++] = 'm';
				}

				inside_escape = 1;

				i++;
				continue;
			}
		}

		if (s[i] == '}' && inside_escape) {
			if (!suppress_colours && (sizeof(buf)-n > 4)) {
				memcpy(buf+n, "\033[0m", 4);
				n += 4;
			}

			inside_escape = 0;
			continue;
		}

		buf[n++] = s[i];
	}

	buf[n] = 0;

	return buf;
}

/* Strip keyd color markup (r{...}, g{...}, etc.) and ANSI escape sequences,
 * returning the plain text content. Result is in a static buffer. */
static const char *strip_markup(const char *s)
{
	static char buf[1024];
	size_t n = 0;
	int inside_escape = 0;
	int inside_ansi = 0;

	for (int i = 0; s[i] && n < sizeof(buf) - 1; i++) {
		/* Skip ANSI escape sequences (\033[...m) */
		if (s[i] == '\033' && s[i+1] == '[') {
			inside_ansi = 1;
			i++;
			continue;
		}
		if (inside_ansi) {
			if (s[i] == 'm')
				inside_ansi = 0;
			continue;
		}

		/* Skip keyd color markup */
		if (!inside_escape && s[i+1] == '{') {
			switch (s[i]) {
			case 'r': case 'g': case 'y': case 'b':
			case 'm': case 'c': case 'w':
				inside_escape = 1;
				i++;
				continue;
			}
		}
		if (inside_escape && s[i] == '}') {
			inside_escape = 0;
			continue;
		}

		buf[n++] = s[i];
	}

	/* Trim trailing newline */
	while (n > 0 && (buf[n-1] == '\n' || buf[n-1] == '\r'))
		n--;

	buf[n] = 0;
	return buf;
}

/* Emit a JSON log line to stderr.
 * Extracts module from the "MODULE: " prefix if present. */
static void emit_json(int level, const char *plain)
{
	const char *module = "";
	const char *msg = plain;
	char module_buf[32] = {0};

	/* Extract "MODULE: " prefix */
	const char *colon = strstr(plain, ": ");
	if (colon && (colon - plain) < (int)sizeof(module_buf) - 1) {
		int all_upper = 1;
		for (const char *p = plain; p < colon; p++) {
			if (*p < 'A' || *p > 'Z') { all_upper = 0; break; }
		}
		if (all_upper && colon > plain) {
			size_t mlen = (size_t)(colon - plain);
			memcpy(module_buf, plain, mlen);
			module_buf[mlen] = 0;
			module = module_buf;
			msg = colon + 2;
		}
	}

	/* JSON-escape msg: replace " with \" and \ with \\ */
	char escaped[1024];
	size_t j = 0;
	for (size_t i = 0; msg[i] && j < sizeof(escaped) - 3; i++) {
		if (msg[i] == '"' || msg[i] == '\\')
			escaped[j++] = '\\';
		escaped[j++] = msg[i];
	}
	escaped[j] = 0;

	fprintf(stderr, "{\"level\":%d,\"module\":\"%s\",\"msg\":\"%s\"}\n",
		level, module, escaped);
}

void die(const char *fmt, ...) {
	fprintf(stderr, "%s", colorize("r{FATAL ERROR:} "));

	va_list ap;
	va_start(ap, fmt);
	vfprintf(stderr, colorize(fmt), ap);
	va_end(ap);
	fprintf(stderr, "\n");

	exit(-1);
}

void _vkeyd_log(const char *fmt, va_list ap)
{
	pthread_mutex_lock(&mtx);
	if (log_format == LOG_FORMAT_JSON) {
		char rendered[1024];
		vsnprintf(rendered, sizeof(rendered), colorize(fmt), ap);
		emit_json(0, strip_markup(rendered));
	} else {
		vprintf(colorize(fmt), ap);
	}
	pthread_mutex_unlock(&mtx);
}

void _keyd_log(int level, const char *fmt, ...)
{
	if (level > log_level)
		return;

	va_list ap;
	va_start(ap, fmt);
	if (log_format == LOG_FORMAT_JSON) {
		char rendered[1024];
		vsnprintf(rendered, sizeof(rendered), colorize(fmt), ap);
		pthread_mutex_lock(&mtx);
		emit_json(level, strip_markup(rendered));
		pthread_mutex_unlock(&mtx);
	} else {
		_vkeyd_log(fmt, ap);
	}
	va_end(ap);
}
