/*
 * keyd - A key remapping daemon.
 *
 * © 2019 Raheman Vaiya (see also: LICENSE).
 */
#ifndef PLATFORM_H
#define PLATFORM_H

/*
 * Platform contract: each platform backend must implement all symbols declared
 * or included in this header.
 *
 * Linux backend:  src/device.c + src/evloop.c + src/vkbd/uinput.c
 * macOS backend:  src/macos/input.c + src/vkbd/macos.c
 */

#include "device.h"
#include "vkbd.h"

struct event; /* full definition in keyd.h */

/*
 * Run the main event loop, calling handler(ev, ctx) for each event.
 * Returns the next timeout in milliseconds, or 0 for no timeout.
 */
int evloop(int (*handler)(struct event *, void *), void *ctx);

/*
 * Register an additional file descriptor to be monitored by the event loop.
 * Activity on fd generates an EV_FD_ACTIVITY event; errors generate EV_FD_ERR.
 */
void evloop_add_fd(int fd);

#endif
