/*
 * Unit tests for config parsing (config.c / config_parse.c).
 *
 * Output: TAP (Test Anything Protocol) format.
 * Run: make test-config  (no root or uinput required)
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../src/keyd.h"
#include "../src/config_parse.h"

static int test_count = 0;
static int pass_count = 0;

static void tap_result(int ok, const char *name, const char *msg)
{
	test_count++;
	if (ok) {
		pass_count++;
		printf("ok %d - %s\n", test_count, name);
	} else {
		printf("not ok %d - %s\n", test_count, name);
		if (msg)
			printf("  ---\n  message: '%s'\n  ...\n", msg);
	}
}

#define PASS(name)           tap_result(1, name, NULL)
#define FAIL(name, msg)      tap_result(0, name, msg)
#define CHECK(cond, name, msg) tap_result(!!(cond), name, (cond) ? NULL : (msg))

/* Returns a config initialised with the default layers (control, shift, etc.)
 * but no device bindings. Uses a minimal [ids] * section so parse succeeds. */
static int make_base_config(struct config *cfg)
{
	return config_parse_string(cfg,
		"[ids]\n"
		"*\n"
		"\n"
		"[main]\n");
}

/* ── descriptor tests ─────────────────────────────────────────────────────── */

static void test_descriptor_basic_keysequence(void)
{
	struct config cfg;
	struct descriptor d;
	struct parse_ctx ctx = {0};
	char s[] = "a";

	if (make_base_config(&cfg) < 0) {
		FAIL("descriptor/basic-keysequence", "config init failed");
		return;
	}

	if (config_parse_descriptor(s, &d, &cfg, &ctx) < 0) {
		FAIL("descriptor/basic-keysequence", "parse failed");
		return;
	}

	CHECK(d.op == OP_KEYSEQUENCE &&
	      d.keysequence.code == KEYD_A &&
	      d.keysequence.mods == 0,
	      "descriptor/basic-keysequence",
	      "expected OP_KEYSEQUENCE with code KEYD_A, mods 0");
}

static void test_descriptor_with_modifier(void)
{
	struct config cfg;
	struct descriptor d;
	struct parse_ctx ctx = {0};
	char s[] = "C-a";

	if (make_base_config(&cfg) < 0) {
		FAIL("descriptor/with-modifier", "config init failed");
		return;
	}

	if (config_parse_descriptor(s, &d, &cfg, &ctx) < 0) {
		FAIL("descriptor/with-modifier", "parse failed");
		return;
	}

	CHECK(d.op == OP_KEYSEQUENCE &&
	      d.keysequence.code == KEYD_A &&
	      (d.keysequence.mods & MOD_CTRL),
	      "descriptor/with-modifier",
	      "expected OP_KEYSEQUENCE with MOD_CTRL set");
}

static void test_descriptor_overload(void)
{
	struct config cfg;
	struct descriptor d;
	struct parse_ctx ctx = {0};
	char s[] = "overload(control, a)";
	int control_idx;

	if (make_base_config(&cfg) < 0) {
		FAIL("descriptor/overload", "config init failed");
		return;
	}

	control_idx = config_get_layer_index(&cfg, "control");
	if (control_idx < 0) {
		FAIL("descriptor/overload", "control layer not found in base config");
		return;
	}

	if (config_parse_descriptor(s, &d, &cfg, &ctx) < 0) {
		FAIL("descriptor/overload", "parse failed");
		return;
	}

	CHECK(d.op == OP_OVERLOAD &&
	      d.overload.layer_idx == control_idx,
	      "descriptor/overload",
	      "expected OP_OVERLOAD pointing to control layer");
}

static void test_descriptor_invalid_key(void)
{
	struct config cfg;
	struct descriptor d;
	struct parse_ctx ctx = {0};
	char s[] = "notavalidkey";
	int ret;

	if (make_base_config(&cfg) < 0) {
		FAIL("descriptor/invalid-key", "config init failed");
		return;
	}

	ret = config_parse_descriptor(s, &d, &cfg, &ctx);

	CHECK(ret < 0,
	      "descriptor/invalid-key",
	      "expected -1 for unknown key name");
}

static void test_descriptor_deprecated_warns(void)
{
	struct config cfg;
	struct descriptor d;
	struct parse_ctx ctx = {0};
	char s[] = "overload2(control, a, 300)";  /* overload2 = deprecated overloadt: needs 3 args */
	int ret;

	if (make_base_config(&cfg) < 0) {
		FAIL("descriptor/deprecated-warns", "config init failed");
		return;
	}

	ret = config_parse_descriptor(s, &d, &cfg, &ctx);

	CHECK(ret == 0 && ctx.nr_warnings == 1,
	      "descriptor/deprecated-warns",
	      "expected success with exactly 1 deprecation warning");
}

/* ── macro tests ──────────────────────────────────────────────────────────── */

static void test_macro_basic_keysequence(void)
{
	struct macro macro = {0};
	int ret = config_parse_macro_expression("C-h", &macro);

	CHECK(ret == 0 && macro.sz > 0 &&
	      macro.entries[0].type == MACRO_KEYSEQUENCE,
	      "macro/basic-keysequence",
	      "expected at least one MACRO_KEYSEQUENCE entry for C-h");
}

static void test_macro_unicode(void)
{
	struct macro macro = {0};
	/* 😄 is U+1F604; macro_parse should produce MACRO_UNICODE entries */
	int ret = config_parse_macro_expression("😄", &macro);

	CHECK(ret == 0 && macro.sz > 0,
	      "macro/unicode",
	      "expected non-empty macro for unicode character 😄");
}

/* ── full-config tests ────────────────────────────────────────────────────── */

static void test_config_minimal_valid(void)
{
	struct config cfg;
	int ret;

	ret = config_parse_string(&cfg,
		"[ids]\n"
		"*\n"
		"\n"
		"[main]\n"
		"a = b\n");

	if (ret < 0) {
		FAIL("config/minimal-valid", "config_parse_string returned failure");
		return;
	}

	/* The main layer (index 0) should have 'a' mapped to something. */
	int main_idx = config_get_layer_index(&cfg, "main");
	if (main_idx < 0) {
		FAIL("config/minimal-valid", "main layer not found");
		return;
	}

	struct descriptor *d = &cfg.layers[main_idx].keymap[KEYD_A];
	CHECK(d->op == OP_KEYSEQUENCE && d->keysequence.code == KEYD_B,
	      "config/minimal-valid",
	      "expected a->b keysequence in main layer");
}

static void test_config_global_section(void)
{
	struct config cfg;
	int ret;

	ret = config_parse_string(&cfg,
		"[ids]\n"
		"*\n"
		"\n"
		"[global]\n"
		"chord_timeout = 75\n"
		"\n"
		"[main]\n");

	CHECK(ret >= 0 && cfg.chord_interkey_timeout == 75,
	      "config/global-section",
	      "expected chord_interkey_timeout == 75 after parsing global section");
}

static void test_config_unknown_global_warns(void)
{
	struct config cfg;
	int ret;

	ret = config_parse_string(&cfg,
		"[ids]\n"
		"*\n"
		"\n"
		"[global]\n"
		"not_a_real_option = 1\n"
		"\n"
		"[main]\n");

	/* config_parse_string returns nr_warnings on partial success */
	CHECK(ret > 0,
	      "config/unknown-global-warns",
	      "expected at least one warning for unrecognised global option");
}

/* ── device-match tests ───────────────────────────────────────────────────── */

static void test_match_wildcard(void)
{
	struct config cfg;

	config_parse_string(&cfg,
		"[ids]\n"
		"*\n"
		"\n"
		"[main]\n");

	int r = config_check_match(&cfg, "1234:5678", ID_KEYBOARD | ID_KEY);

	CHECK(r > 0,
	      "match/wildcard",
	      "expected nonzero match rank for wildcard config");
}

static void test_match_exact(void)
{
	struct config cfg;

	config_parse_string(&cfg,
		"[ids]\n"
		"1234:5678\n"
		"\n"
		"[main]\n");

	int r_exact   = config_check_match(&cfg, "1234:5678", ID_KEYBOARD | ID_KEY);
	int r_nomatch = config_check_match(&cfg, "0000:0000", ID_KEYBOARD | ID_KEY);

	CHECK(r_exact == 2 && r_nomatch == 0,
	      "match/exact",
	      "expected rank 2 for exact match, 0 for non-matching ID");
}

static void test_match_exclusion(void)
{
	struct config cfg;

	config_parse_string(&cfg,
		"[ids]\n"
		"*\n"
		"-1234:5678\n"
		"\n"
		"[main]\n");

	int r = config_check_match(&cfg, "1234:5678", ID_KEYBOARD | ID_KEY);

	CHECK(r == 0,
	      "match/exclusion",
	      "expected 0 for excluded device ID");
}

/* ── config_add_entry tests ───────────────────────────────────────────────── */

static void test_add_entry_valid(void)
{
	struct config cfg;

	config_parse_string(&cfg,
		"[ids]\n"
		"*\n"
		"\n"
		"[main]\n");

	int ret = config_add_entry(&cfg, "main.a = b");

	int main_idx = config_get_layer_index(&cfg, "main");
	struct descriptor *d = &cfg.layers[main_idx].keymap[KEYD_A];

	CHECK(ret == 0 && d->op == OP_KEYSEQUENCE && d->keysequence.code == KEYD_B,
	      "config/add-entry-valid",
	      "expected a->b binding after config_add_entry");
}

static void test_add_entry_bad_layer(void)
{
	struct config cfg;

	config_parse_string(&cfg,
		"[ids]\n"
		"*\n"
		"\n"
		"[main]\n");

	int ret = config_add_entry(&cfg, "nonexistent_layer.a = b");

	CHECK(ret < 0,
	      "config/add-entry-bad-layer",
	      "expected -1 for binding to a nonexistent layer");
}

/* ── main ─────────────────────────────────────────────────────────────────── */

int main(void)
{
	int total = 15;

	printf("1..%d\n", total);

	test_descriptor_basic_keysequence();
	test_descriptor_with_modifier();
	test_descriptor_overload();
	test_descriptor_invalid_key();
	test_descriptor_deprecated_warns();

	test_macro_basic_keysequence();
	test_macro_unicode();

	test_config_minimal_valid();
	test_config_global_section();
	test_config_unknown_global_warns();

	test_match_wildcard();
	test_match_exact();
	test_match_exclusion();

	test_add_entry_valid();
	test_add_entry_bad_layer();

	if (pass_count == test_count) {
		printf("\n# All %d tests passed.\n", test_count);
		return 0;
	} else {
		printf("\n# %d/%d tests passed.\n", pass_count, test_count);
		return 1;
	}
}
