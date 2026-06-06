/*
 * keyd - A key remapping daemon.
 *
 * © 2019 Raheman Vaiya (see also: LICENSE).
 */

#include "../../keyd.h"
#include "../../platform.h"

static long linux_get_time_ms(void)
{
	struct timespec ts;
	clock_gettime(CLOCK_MONOTONIC, &ts);
	return ts.tv_sec * 1E3 + ts.tv_nsec / 1E6;
}

static void linux_set_realtime(void)
{
	struct sched_param sp;
	sp.sched_priority = 49;
	if (sched_setscheduler(0, SCHED_FIFO, &sp)) {
		perror("sched_setscheduler");
	}
}

static void linux_lock_memory(void)
{
	if (mlockall(MCL_CURRENT | MCL_FUTURE)) {
		perror("mlockall");
	}
}

static const struct platform linux_platform = {
	.get_time_ms = linux_get_time_ms,
	.set_realtime = linux_set_realtime,
	.lock_memory = linux_lock_memory,

	.device_scan = device_scan,
	.device_grab = device_grab,
	.device_ungrab = device_ungrab,
	.device_set_led = device_set_led,

	.evloop_add_fd = evloop_add_fd,
	.evloop = evloop,

	.device_read_event = device_read_event,

	.vkbd_init = vkbd_init,

	.ipc_create_server = ipc_create_server,
	.ipc_connect = ipc_connect,
};

const struct platform *platform = &linux_platform;
