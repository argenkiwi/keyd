#include "../../keyd.h"

int evloop(int (*event_handler) (struct event *ev))
{
	return platform->evloop(event_handler);
}

void evloop_add_fd(int fd)
{
	platform->evloop_add_fd(fd);
}
