/*
 * keyd - A key remapping daemon.
 *
 * © 2019 Raheman Vaiya (see also: LICENSE).
 */
#ifndef PLATFORM_H
#define PLATFORM_H

#include <stdint.h>
#include <stddef.h>
#include "device.h"
#include "vkbd.h"

struct event;

struct platform {
	/* System */
	long (*get_time_ms)(void);
	void (*set_realtime)(void);
	void (*lock_memory)(void);

	/* Device Management */
	int (*device_scan)(struct device *devices);
	int (*device_grab)(struct device *dev);
	int (*device_ungrab)(struct device *dev);
	void (*device_set_led)(const struct device *dev, int led, int state);

	/* Event Loop */
	void (*evloop_add_fd)(int fd);
	int (*evloop)(int (*event_handler) (struct event *ev));

	/* Input */
	struct device_event *(*device_read_event)(struct device *dev);

	/* Output (Virtual Device) */
	struct vkbd *(*vkbd_init)(const char *name);

	/* IPC */
	int (*ipc_create_server)(void);
	int (*ipc_connect)(void);
};

extern const struct platform *platform;

/* Introspection API (Core logic, but exposed via PAL/IPC) */
struct keyboard;
void platform_get_kbd_state(const struct keyboard *kbd, char *buf, size_t sz);

#endif
