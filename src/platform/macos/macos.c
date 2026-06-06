/*
 * keyd - A key remapping daemon.
 *
 * © 2019 Raheman Vaiya (see also: LICENSE).
 */

#include "../../keyd.h"
#include "../../platform.h"

/* Stub implementation for macOS to allow compilation of common code */

static long macos_get_time_ms(void)
{
	/* TODO: Implement using mach_absolute_time */
	return 0;
}

static void macos_set_realtime(void)
{
	/* TODO: Implement using thread_policy_set */
}

static void macos_lock_memory(void)
{
	/* TODO: Implement using mlockall or equivalent */
}

static int macos_device_scan(struct device *devices)
{
	/* TODO: Implement using IOKit */
	return 0;
}

static int macos_device_grab(struct device *dev)
{
	return 0;
}

static int macos_device_ungrab(struct device *dev)
{
	return 0;
}

static void macos_device_set_led(const struct device *dev, int led, int state)
{
}

static void macos_evloop_add_fd(int fd)
{
}

static int macos_evloop(int (*event_handler) (struct event *ev))
{
	/* TODO: Implement using CFRunLoop */
	return 0;
}

static struct device_event *macos_device_read_event(struct device *dev)
{
	return NULL;
}

static struct vkbd *macos_vkbd_init(const char *name)
{
	return NULL;
}

static int macos_ipc_create_server(void)
{
	return ipc_create_server();
}

static int macos_ipc_connect(void)
{
	return ipc_connect();
}

static const struct platform macos_platform = {
	.get_time_ms = macos_get_time_ms,
	.set_realtime = macos_set_realtime,
	.lock_memory = macos_lock_memory,

	.device_scan = macos_device_scan,
	.device_grab = macos_device_grab,
	.device_ungrab = macos_device_ungrab,
	.device_set_led = macos_device_set_led,

	.evloop_add_fd = macos_evloop_add_fd,
	.evloop = macos_evloop,

	.device_read_event = macos_device_read_event,

	.vkbd_init = macos_vkbd_init,

	.ipc_create_server = macos_ipc_create_server,
	.ipc_connect = macos_ipc_connect,
};

const struct platform *platform = &macos_platform;
