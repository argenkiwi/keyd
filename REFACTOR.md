# keyd Refactoring Roadmap

This document provides a structured plan to refactor the keyd codebase based on the architectural improvements outlined in the analysis.
Each phase is self-contained where possible, with clear dependencies between stages.

---

## Phase 0 — Foundation: Split the `keyd.h` Umbrella Header

**Priority**: P0  |  **Effort**: Low  |  **Risk**: Medium

**Goals**

- Break the single massive `keyd.h` that transitively includes every header
- Give each module its own minimal include set
- Enable incremental compilation improvements

**Steps**

1. **Audit header dependencies**

   For each `src/*.c`, determine which headers are actually needed.

   ```bash
   for f in src/*.c; do
       grep '#include' $f | sort -u
   done
   ```

2. **Create a new `src/base.h`** with only truly shared, system-wide types:

   ```c
   /* src/base.h - minimal common definitions */
   #include <stdint.h>
   #include <stddef.h>

   #define ARRAY_SIZE(x)  (sizeof(x) / sizeof((x)[0]))
   #define VKBD_NAME      "keyd virtual keyboard"
   #define MAX_IPC_MESSAGE_SIZE 4096
   ```

3. **Create `src/ipc.h`** with the IPC message format:

   ```c
   /* src/ipc.h */
   #include <stdint.h>
   #include "base.h"

   enum ipc_type {
       IPC_SUCCESS, IPC_FAIL,
       IPC_BIND, IPC_INPUT, IPC_MACRO,
       IPC_RELOAD, IPC_LAYER_LISTEN,
       IPC_QUERY,         /* future: self-documentation */
   };

   struct ipc_message {
       enum ipc_type type;
       uint32_t timeout;
       size_t data_len;
       char data[MAX_IPC_MESSAGE_SIZE];
   };
   ```

4. **Create `src/event.h`** with the event-loop types:

   ```c
   /* src/event.h */
   #include <stdint.h>
   #include <poll.h>
   #include "keyd.h"   /* forward-declare device */

   enum event_type {
       EV_DEV_ADD, EV_DEV_REMOVE,
       EV_DEV_EVENT, EV_FD_ACTIVITY, EV_FD_ERR,
       EV_TIMEOUT,
   };

   struct event {
       enum event_type type;
       struct device *dev;
       struct device_event *devev;
       int timestamp;
       int fd;
   };
   ```

5. **Update each `.c` file** to replace `#include "keyd.h"` with its minimal needed headers.
   Example transformations:

   | Old | New |
   |---|---|
   | `daemon.c`: `#include "keyd.h"` (everything) | `#include "keyd.h"` (keep, but it becomes lean), `#include "vkbd.h"`, `#include "config.h"`, `#include "ipc.h"` |
   | `check.c`: `#include "keyd.h"` | `#include "config.h"`, `#include "ipc.h"` |
   | `device.c`: system includes | system includes only (remove `keyd.h`) |
   | `evloop.c`: `#include "keyd.h"` | `#include "event.h"`, `#include "device.h"` |
   | `monitor.c`: `#include "keyd.h"` | `#include "device.h"`, `#include "event.h"`, `#include "log.h"` |
   | `macro.c`: `#include "keyd.h"` | `#include "macro.h"`, `#include "config.h"` |
   | `keys.c`: no `keyd.h` → keep as-is | no change needed |
   | `unicode.c`: no `keyd.h` → keep as-is | no change needed |

6. **Break circular dependency risk**:
   - Note: `keyboard.h` currently includes `device.h` and `config.h`. Ensure `device.h` does not transitively include `keyboard.h`.
   - `device.h` has `struct device` with `void *data`. If it needs `keyboard.h` types, use forward declarations instead.

7. **Verify**: Rebuild with `make clean && make`. Run `make test-io`. All tests must still pass.

**Deliverables**

- `src/base.h` — minimal shared definitions
- `src/ipc.h` — IPC types
- `src/event.h` — event loop types
- All `src/*.c` files with updated includes
- No compilation warnings

---

## Phase 1 — Split `keyboard.c` into Engine Sub-Modules

**Priority**: P0  |  **Effort**: Medium  |  **Risk**: High

**Goals**

- Decompose 1,310-line `keyboard.c` into focused, testable modules
- Make the layer resolution pipeline explicit
- Reduce the "all-in-one" state machine coupling

### 1.1 Directory Layout

```
src/engine/
  engine.h              # Public facade: keyboard operations
  engine.c              # Main event dispatch, state orchestration (~200 lines)
  layers.h / .c         # Layer resolution stack (~250 lines)
  overloads.h / .c      # Tap-vs-hold, overload, overloadt (~200 lines)
  oneshot.h / .c        # Oneshot latch, timer, release (~100 lines)
  chords.h / .c         # Chord debouncing, disambiguation (~150 lines)
  cache.h / .c          # Descriptor cache for mid-stroke layer swaps (~80 lines)
  timeouts.h / .c       # timeout() action tracking, active_macro (~100 lines)
  scroll.h / .c         # Scroll remapping state (~60 lines)
```

### 1.2 Extract Steps

**Step A: Layer Resolution**

- Extract `resolve_layer()` and the layer-stack walk logic into `layers.c`
- `layers.h` exposes: `layer_resolve(...)` returning a `struct descriptor` or `NULL`
- Keep in `engine.c` the per-keyboard state (layer_state array, last_repeatable_action)

**Step B: Overload Resolution**

- Extract `pending_overload` struct and its state machine into `overloads.c`
- `overloads.h` exposes: `overload_tap(...)`, `overload_resolve(...)`, `overload_cancel(...)`

**Step C: Oneshot**

- Extract `oneshot_latch`, `oneshot_timeout`, and the `KEYD_ONESHOT_LATCHED` path into `oneshot.c`
- `oneshot.h` exposes: `oneshot_activate()`, `oneshot_deactivate()`, `oneshot_is_active()`

**Step D: Chords**

- Extract the chord resolution state machine (`CHORD_RESOLVING`, `CHORD_INACTIVE`, etc.) into `chords.c`
- `chords.h` exposes: `chord_track_key()`, `chord_resolve()`, `chord_clear()`

**Step E: Cache**

- Extract `cache_set`, `cache_get`, and the `struct cache_entry` into `cache.c`
- `cache.h` exposes: `cache_put()`, `cache_get()`, `cache_drop()`

**Step F: Timeouts**

- Extract the `pending_timeout` state machine and `timeouts[]` table into `timeouts.c`
- `timeouts.h` exposes: `timeout_set()`, `timeout_check()`, `timeout_clear()`, `macro_resolve()`

**Step G: Scroll**

- Extract the `struct scroll` state and `KEYD_SCROLL_*` dispatch into `scroll.c`
- `scroll.h` exposes: `scroll_activate()`, `scroll_deactivate()`, `scroll_dispatch()`

**Deliverables**

- Full `src/engine/` directory with all 9 files
- Original `keyboard.c` reduced to ~200 lines of dispatch glue
- `make` and `make test-io` verified

### 1.3 Dependency Graph After Refactor

```
engine.h ──┐
engine.c   ├──> all sub-module headers
layers.h   │
overloads.h│
chords.h   │
```

No cross-dependencies among sub-module headers. Each is independently testable.

---

## Phase 2 — Split `config.c` into Focused Modules

**Priority**: P1  |  **Effort**: Medium  |  **Risk**: Medium

**Goals**

- Decompose the 1,134-line monolith into parsing, include resolution, validation, and matching
- Make config parsing reentrant / thread-safe (eliminate global `current_file`, `current_line`)

### 2.1 Directory Layout

```
src/config/
  config.h              # Public API: config_parse(), config_check_match()
  parser.h / .c         # Low-level INI tokenizing (delegates to ini.c)
  include.h / .c        # Include directive resolution, cycle detection
  resolver.h / .c       # Layer/alias/descriptor name resolution
  validation.h / .c     # Binding validation (used by `keyd check`)
  match.h / .c          # Device ID matching against config [ids] sections
  srcmap.h / .c         # Source map tracking for error messages
```

### 2.2 Extract Steps

**Step A: Parser**

- Move the INI parsing into `parser.c` via functions on a `struct parser_context`
- Replace globals `current_file`, `current_line`, `nr_warnings` with context fields

```c
struct parser_context {
    const char *path;
    FILE *fp;
    struct srcmap srcmap;
    char error[256];
    int line;
    int warnings;
};
```

**Step B: Include Resolution**

- Extract the `include` directive walker into `include.c`
- Add cycle detection (track visited paths in a set)
- Handle recursive includes up to `MAX_INCLUDES=128` depth

**Step C: Resolver**

- Move alias resolution (`aliases[...]`) and descriptor lookup into `resolver.c`
- Use a `struct layer_context` for layer name → index mapping

**Step D: Validation**

- Take `config_check_match()` logic and bind-validation from the old `config.c`
- Add `keyd check`-specific diagnostics (unused bindings, unknown keys, etc.)

**Step E: Match**

- Move `[ids]` section parsing and `config_check_match()` into `match.c`
- Device type flags (`CAP_KEYBOARD`, `ID_EXCLUDED`, etc.) live together

**Deliverables**

- Full `src/config/` directory
- `config.h` public API unchanged (same entry points for external consumers)
- `make test-io` verified

---

## Phase 3 — Split `daemon.c` CLI and Daemon Concerns

**Priority**: P2  |  **Effort**: Medium  |  **Risk**: Low

**Goals**

- Separate the daemon's event loop + device handling from `keyd.c`'s CLI subcommands
- Make each CLI subcommand a self-contained module

### 3.1 Directory Layout

```
src/daemon/
  daemon.h              # Daemon entry: daemon_run()
  daemon.c              # Event loop, device scan, input grab/release, IPC server
  vkbd_reg.h / .c       # Virtual keyboard backend plugin registration

src/cli/
  cli.h / .c            # Argument parsing, subcommand dispatch
  commands/
    bind.c              # keyd bind <binding>
    input.c             # keyd input <keys>
    do.c                # keyd do <action>
    monitor.c           # keyd monitor (now lives as a daemon too)
    check.c             # keyd check (delegates to config/validation)
    listen.c            # keyd listen
    show.c              # keyd show
```

### 3.2 Extract Steps

**Step A: Daemon Event Loop**

- Move `daemon.c`'s `run_daemon()` into `src/daemon/daemon.c`
- Extract IPC server loop into a standalone `ipc_server.c` in `src/daemon/`
- The daemon uses `vkbd_reg.h` to find the backend at runtime

**Step B: CLI Dispatch**

- `src/cli/cli.c` handles `argc/argv` routing:
  - `keyd monitor` → spawns a client that reads from socket
  - `keyd check` → delegates to `config/validation.h`
  - `keyd bind / input / do` → IPC via `ipc_connect()`
  - `keyd listen` → one-shot IPC read

**Step C: IPC Server**

- Factor `ipc_create_server()`, the bind + accept loop into `src/daemon/ipc_server.c`
- Use the plugin interface from Phase 4 for Vkbd lookup

**Deliverables**

- `src/cli/` and `src/daemon/` directories
- `make` produces `bin/keyd` with all CLI subcommands working
- All IPC-based subcommands tested

---

## Phase 4 — Runtime Vkbd Backend Dispatch

**Priority**: P1  |  **Effort**: Low  |  **Risk**: Low

**Goals**

- Replace compile-time `VKBD=` selection with runtime backend choice
- Enable `keyd show-vkbd` to list available backends
- Allow `keyd -b uinput` or `KEYD_VKBD=uinput` env var override

### 4.1 The Plugin Interface

```c
/* src/vkbd/vkbd_reg.h */
struct vkbd_backend {
    const char *name;            // "uinput", "stdout", "usb-gadget"
    uint32_t flags;              // e.g., VKBD_FLAG_REQUIRES_ROOT
    struct vkbd *(*init)(const char *dev_name, const char *args);
    void (*destroy)(struct vkbd *v);
};

#define register_backend(backend) \
    __attribute__((constructor)) static void _init_##backend(void) \
    { backend_table[backend_table_sz++] = &(backend_instance_##backend); }

static struct vkbd_backend *backend_table[8];
static size_t backend_table_sz = 0;
```

### 4.2 Extract Steps

**Step A: Create `vkbd_reg.c`**

- Maintain a list of registered backends
- Add `vkbd_find_backend(const char *name) → const struct vkbd_backend *`

**Step B: Convert `uinput.c` → `src/vkbd/uinput.c`**

- Wrap initialization/destroy in the plugin interface
- Move to `src/vkbd/` subdirectory

```c
struct vkbd_backend vkbd_backend_uinput = {
    .name = "uinput",
    .flags = VKBD_FLAG_REQUIRES_ROOT,
    .init = vkbd_uinput_init,
    .destroy = vkbd_uinput_destroy,
};
register_backend(vkbd_backend_uinput)
```

**Step C: Convert `stdout.c` and `usb-gadget.c`**

- Same pattern, different device targets

**Step D: Update `daemon.c` → `daemon/daemon.c`**

- Replace `vkbd_init()` with `vkbd_find_backend()` lookup
- Read `KEYD_VKBD` env var for override
- Add `keyd show-vkbd` CLI to list available backends

**Deliverables**

- `src/vkbd/vkbd_reg.h` / `.c` plugin infrastructure
- Backends: `uinput`, `stdout`, `usb-gadget` all as plugins
- `KEYD_VKBD=uinput` env-var overrides
- `make test-io` passes

---

## Phase 5 — Dynamic Sizing for Bounded Arrays

**Priority**: P2  |  **Effort**: Medium  |  **Risk**: Medium

**Goals**

- Replace static `MAX_LAYERS=32`, `MAX_DEVICES=64`, `CACHE_SIZE=16`, etc. with dynamic arrays where feasible
- Reduce binary size and eliminate silent truncation

### 5.1 Targets for Dynamic Allocation

| Current Static | Suggested Dynamic | Rationale |
|---|---|---|
| `layer_state[MAX_LAYERS]` | Growable array in `struct keyboard` | Layers created at runtime from config |
| `timeouts[128]` | Growable array | Timeouts per-keyboard, not per-compilation |
| `chords[64]` per layer | Growable array | Depends on config file |
| `keymap[256]` | Keep as-is | Evdev codes are a fixed 0–255 range |
| `keystate[256]` | Keep as-is | Evdev codes are fixed range |
| `macros[256]` | Growable array | From config file |
| `descriptors[1024]` | Growable array | From config file |

### 5.2 Extract Steps

**Step A: Growable Array Helpers**

Create `src/util/array.h` / `.c`:

```c
struct dyn_array {
    void *elements;
    size_t elem_size;
    size_t count;
    size_t capacity;
};

int da_append(struct dyn_array *arr, const void *elem);
void *da_get(const struct dyn_array *arr, size_t idx);
void da_free(struct dyn_array *arr);
```

**Step B: Apply to `struct keyboard` (engine/engine.c)**

```c
// Before:
struct { long activation_time; uint8_t active; ... } layer_state[MAX_LAYERS];

// After:
struct dyn_array layer_states;  // each element is struct layer_state_entry
```

**Step C: Apply to `timeouts[]`, `chords[]`, `macros[]`, `descriptors[]`**

Similar pattern — each becomes a `struct dyn_array` in the appropriate struct.

**Deliverables**

- `src/util/array.h` / `.c` reusable dynamic array helpers
- `struct keyboard` with dynamic `layer_states`, `timeouts`, etc.
- `make test-io` verified on all test inputs
- Binary size reduced / config flexibility improved

---

## Phase 6 — Unicode Data as MMAP'd Binary Blob

**Priority**: P2  |  **Effort**: Low  |  **Risk**: Low

**Goals**

- Remove ~21,000 lines of auto-generated code from the compilation unit
- Improve build times and reduce binary overhead via shared mapping

### 6.1 Extract Steps

**Step A: Generate a Compiled Table Binary**

Modify `scripts/generate_xcompose` to produce both:

1. `data/keyd.compose` — human-readable (existing format)
2. `build/unicode_table.bin` — compiled binary lookup table

The binary format is simple:

```
[4B: key count] [N * {4B: key, 4B: length, lengthB: sequence}]
```

Or even simpler: a compressed `malloc`-able blob with an offset table.

**Step B: Build-time Generation**

```makefile
build/unicode_table.bin: scripts/generate_xcompose tools/compose_to_bin
	$< | $^ > $@

tools/compose_to_bin:
	$(CC) -o $@ tools/compose_to_bin.c
```

**Step C: Runtime Loading**

```c
/* src/unicode.h */
extern const uint8_t unicode_table_data[];
extern const size_t unicode_table_data_size;

struct unicode_entry *unicode_lookup(uint8_t key);  // uses mmap'd blob
```

In `unicode.c`:

```c
#include <sys/mman.h>

#ifdef HAVE_MMAP_BLOB
#include "build/unicode_table.bin"

struct unicode_entry *unicode_lookup(uint8_t key) {
    const uint8_t *blob = __builtin_choose_expr(
        sizeof(unicode_table_data) > 0,
        unicode_table_data,
        (const uint8_t *)fallback_blob
    );
    /* binary search on mmap'd data */
}
#else
/* fallback: compile the old table */
#endif
```

**Step D: Dual-Mode Fallback**

Keep the old compiled table as `__attribute__((used)) static` for build environments that don't link the blob. This preserves backward compatibility.

**Deliverables**

- `scripts/generate_xcompose` → both `.compose` and `.bin`
- `build/unicode_table.bin` generated at build time
- `tools/compose_to_bin.c` build-time converter
- Dual-mode fallback for non-mmap builds

---

## Phase 7 — Test Harness Improvements

**Priority**: P3  |  **Effort**: Medium  |  **Risk**: Low

**Goals**

- Make the test harness more maintainable and less coupled
- Add a pure-logic test mode (no uinput/root required)
- Reduce monolithic `runner.py`

### 7.1 Directory Layout

```
t/
  harness/
    __init__.py
    vkb.py        # Virtual keybard setup (uinput)
    stream.py     # Key stream / grab/ungrab
    timing.py     # Precise sleep, timeout helpers
  parser.py       # Test file format → TestCase objects
  assert.py       # Expected vs actual diff + reporting
  runner.py       # Orchestrator only (~50 lines)
  keys.py         # Key name ↔ code mappings (unchanged)
  test-io.c       # Compiles from Makefile.test
  Makefile.test   # Separate test compilation
  run.sh          # Entry point (unchanged)
```

### 7.2 Extract Steps

**Step A: Split `runner.py`**

```python
# runner.py (orchestrator, ~60 lines)
import sys
sys.argv.append('--harness')
import harness.harness
import parser
import assert_module
```

**Step B: `harness/vkb.py`**

- VirtualKeyboard → standalone module with `create_vkb(name, vendor_id, product_id)`
- KeyStream → standalone module with `grab()`, `ungrab()`, `collect()`
- `harness/timing.py` → `sleep_us(us)`, `sleep_ms(ms)` helpers

**Step C: `parser.py`**

- Parse test file format → `TestCase(name, inputs, expected_outputs)`
- No test execution logic, just parsing

**Step D: `assert.py`**

- `diff_report(expected, actual)` → formatted diff
- `assert_result(name, expected, actual, verbose)` → printed output + PASS/FAIL
- Test counter and exit-code management

**Step E: Pure-Logic Test Mode**

Add `Makefile.test`:

```makefile
# t/Makefile.test
PANDIR := ../src

.PHONY: test-io test-logic

test-io:
	$(CC) -O2 -o test-io \
		src/keyboard.c src/config.c src/macro.c \
		src/log.c src/ini.c src/keys.c src/unicode.c \
		src/engine/engine.c src/engine/layers.c \
		test/test-main.c  # simplified test runner in C

test-logic: test-io
	./test-io t/*.t   # Still needs uinput for full hardware simulation

# Goal: eliminate dependency on uinput for the keyboard logic tests
```

The `test-main.c` would mock vkbd_send_key and capture output directly into an array, avoiding the need for a real uinput device.

**Deliverables**

- Split `runner.py` into modular components
- `t/Makefile.test` with separate build targets
- Documentation update in `t/README.md`

---

## Phase 8 — Formalize IPC Protocol

**Priority**: P2  |  **Effort**: Low  |  **Risk**: Low

**Goals**

- Publish an IPC specification document
- Add version awareness to the protocol
- Document existing commands and data layout

### 8.1 Extract Steps

**Step A: Create `docs/ipc-protocol.md`**

```markdown
# keyd IPC Protocol

## Socket
- Path: `/var/run/keyd.socket`
- Owner: `keyd` group
- Protocol: Binary, little-endian

## Message Layout

struct ipc_message {
    uint32_t version;        // 1 (current)
    uint32_t type;           // enum ipc_type
    uint32_t timeout_ms;     // 0 = no timeout
    uint32_t data_len;       // bytes in data[]
    char data[MAX_IPC_MESSAGE_SIZE]; // variable length
} __attribute__((packed));

## Command Types

| Type | Timeout | Data Format | Description |
|------|---------|-------------|-------------|
| IPC_BIND | 0 | `struct bind_msg` | Register a key binding |
| IPC_INPUT | 0 | `char keys[N]` | Send raw key sequence |
| IPC_MACRO | 0 | `char name[N]` | Execute named macro |
| IPC_RELOAD | 0 | none | Reload configuration |
| IPC_LAYER_LISTEN | 0 | `char layer_name[N]` | Activate layer listener |

## Data Structures

### bind_msg
```c
struct bind_msg {
    uint8_t key;         // evdev key code
    uint8_t action_len;  // length of action string
    char action[256];    // action expression
};
```

## Client Library

A simple Python client library is provided as `tools/keyd_py/` for
external tool developers.

```python
from keyd_py import Client
c = Client()
c.bind('capslock', 'layer(emacs)')
c.input('C-a')
```
```

**Step B: Add `IPC_QUERY` Command**

In the IPC protocol, add a `IPC_QUERY` variant:

```c
enum ipc_type {
    // ... existing ...
    IPC_QUERY,                  // self-documentation
};
```

Query types:

| Subtype | Response |
|---|---|
| `QUERY_BACKENDS` | list registered Vkbd backends |
| `QUERY_DEVICES` | list active keyboards |
| `QUERY_LAYERS` | list active layers per device |

**Step C: Wire into CLI**

- `keyd show-vkbd` → sends `IPC_QUERY` with `QUERY_BACKENDS`
- `keyd show-devices` → sends `IPC_QUERY` with `QUERY_DEVICES`

**Deliverables**

- `docs/ipc-protocol.md` published specification
- `IPC_QUERY` command implemented
- `keyd show-vkbd` and `keyd show-devices` CLI commands

---

## Cross-Phase Integration Plan

After individual phases are complete, perform a final integration pass:

### A. Unified Build System Upgrade

Adopt `CMakeLists.txt` or improve the `Makefile`:

```makefile
# Minimal Makefile improvements
CFLAGS += -MMD -MP          # auto-dependency tracking
CFLAGS += -Wall -Wextra -Wshadow
CFLAGS += -Werror=format-security

# Object file tracking
SRCS = $(shell find src -name '*.c' | sort)
OBJS = $(SRCS:.c=.o)

bin/keyd: $(OBJS)
	$(CC) $(LDFLAGS) -o $@ $^ -lpthread

%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

-include $(OBJS:.o=.d)      # auto-included dependency files
```

### B. Integration Test Suite

Create `t/integration/`:

```
t/integration/
  Makefile
  basic-test.sh        # daemon start/stop test
  monitor-test.sh      # keyd monitor output
  bind-io-test.sh      # IPC bind + input roundtrip
```

### C. Documentation Updates

- Update `docs/architecture.md` to reflect new directory layout
- Add `REFACTOR.md` to `docs/` as the changelog
- Ensure `t/README.md` reflects the new test harness structure

---

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| Phase 0 breakage (header splitting) | `git bisect`-style testing: compile after every `.c` file change, run `make test-io` |
| Phase 1 complexity (keyboard split) | Extract one sub-module at a time; keep tests green after each |
| Phase 5 dynamic sizing bugs | Add bounds-checking in debug builds: `#ifdef DEBUG assert(idx < count)` |
| Phase 6 mmap blob incompatibility | Dual-mode fallback (compiled table preserved as last resort) |
| Phase 7 test harness breakage | Run old `runner.py` alongside new modules during transition |

---

## Execution Order (Recommended)

```
Phase 0:  Split keyd.h umbrella header     (2-3 days)
Phase 1:  Split keyboard.c engine         (1-2 weeks)
Phase 2:  Split config.c parser            (1-2 weeks)
Phase 3:  Split daemon.c + CLI               (1 week)
Phase 4:  Runtime Vkbd dispatch              (2-3 days)
Phase 5:  Dynamic sizing for arrays          (3-5 days)
Phase 6:  Unicode mmap blob                  (1-2 days)
Phase 8:  Formalize IPC protocol             (1-2 days)
Phase 7:  Test harness improvements          (1 week)
Phase 9:  Integration + docs update          (1 week)
```

**Total estimated effort**: ~6-8 weeks for a small team (2-3 engineers).

---

## Success Criteria

| Metric | Target |
|---|---|
| `keyboard.c` size | < 300 lines (facade) + sub-modules |
| `config.c` size | < 300 lines (facade) + sub-modules |
| `keyd.h` includes | < 5 headers max |
| Build time (cold) | 40% reduction |
| Test harness coupling | Zero hard-coded paths |
| IPC protocol | Documented + versioned |
| Vkbd backends | Runtime selectable, no recompile |
| Dynamic sizing | < 10 hardcoded array constants |
| Unicode bloat | 0 lines in `.c` files |
