/*
 * keyd - A key remapping daemon.
 *
 * © 2019 Raheman Vaiya (see also: LICENSE).
 */
#ifndef KEYD_LOG_H
#define KEYD_LOG_H

#include <stdio.h>
#include <stdarg.h>
#include <string.h>
#include <stdlib.h>

#define keyd_log(fmt, ...) _keyd_log(0, fmt, ##__VA_ARGS__);

/* Module-tagged log helpers — produce "MODULE: message\n" prefixed lines.
 * Use these in place of keyd_log() for output that should be filterable by
 * subsystem (e.g. grep '^CONFIG:' or JSON log parsing). */
#define log_config(fmt, ...) _keyd_log(0, "CONFIG: " fmt  "\n", ##__VA_ARGS__)
#define log_device(fmt, ...) _keyd_log(0, "DEVICE: " fmt  "\n", ##__VA_ARGS__)
#define log_daemon(fmt, ...) _keyd_log(0, "DAEMON: " fmt  "\n", ##__VA_ARGS__)
#define log_kbd(fmt, ...)    _keyd_log(0, "KBD: "    fmt  "\n", ##__VA_ARGS__)

#define dbg(fmt, ...) _keyd_log(1, "r{DEBUG:} b{%s:%d:} "fmt"\n", __FILE__, __LINE__, ##__VA_ARGS__)
#define dbg2(fmt, ...) _keyd_log(2, "r{DEBUG:} b{%s:%d:} "fmt"\n", __FILE__, __LINE__, ##__VA_ARGS__)

#define err(fmt, ...) snprintf(errstr, sizeof(errstr), fmt, ##__VA_ARGS__);

void _keyd_log(int level, const char *fmt, ...);
void _vkeyd_log(const char *fmt, va_list ap);
void die(const char *fmt, ...);

extern int log_level;
extern int suppress_colours;

/* log_format: controls output encoding. Set LOG_FORMAT_JSON to emit
 * machine-readable {"level":N,"module":"X","msg":"..."} lines instead of
 * coloured human text. Read KEYD_LOG_FORMAT=json at daemon startup. */
#define LOG_FORMAT_TEXT 0
#define LOG_FORMAT_JSON 1
extern int log_format;

/*
 * errstr: last error message set by err(). Single-threaded use only.
 * Always read immediately after a function that may call err() — any
 * subsequent library call may overwrite it. Never use across threads.
 */
extern char errstr[8192];

#endif
