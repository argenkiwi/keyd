/*
 * keyd - A key remapping daemon.
 *
 * © 2019 Raheman Vaiya (see also: LICENSE).
 */

#include "keyd.h"

/* TODO (maybe): settle on an API and publish the protocol. */

static int chgid(void)
{
#ifndef __APPLE__
	/* On macOS there is no "keyd" system group; socket is in /tmp. */
	struct group *g = getgrnam("keyd");

	if (!g) {
		fprintf(stderr,
			"WARNING: failed to set effective group to \"keyd\" (make sure the group exists)\n");
	} else {
		if (setgid(g->gr_gid)) {
			err("setgid: %s", strerror(errno));
			return -1;
		}
	}
#endif
	return 0;
}

int ipc_connect(void)
{
	int sd = socket(AF_UNIX, SOCK_STREAM, 0);
	struct sockaddr_un addr = {0};

	if (sd < 0) {
		perror("socket");
		exit(-1);
	}

	addr.sun_family = AF_UNIX;
	strncpy(addr.sun_path, SOCKET_PATH, sizeof(addr.sun_path)-1);

	if (connect(sd, (struct sockaddr *) &addr, sizeof addr) < 0) {
		fprintf(stderr, "ERROR: Failed to connect to \"" SOCKET_PATH "\", make sure the daemon is running and you have permission to access the socket.\n");
		exit(-1);
	}

	return sd;
}

int ipc_create_server(void)
{
	char lockpath[PATH_MAX];
	int sd = socket(AF_UNIX, SOCK_STREAM, 0);
	int lfd;
	struct sockaddr_un addr = {0};

	if (chgid() < 0)
		return -1;

	if (sd < 0) {
		err("socket: %s", strerror(errno));
		return -1;
	}
	addr.sun_family = AF_UNIX;
	strncpy(addr.sun_path, SOCKET_PATH, sizeof(addr.sun_path)-1);
	snprintf(lockpath, sizeof lockpath, "%s.lock", SOCKET_PATH);
	lfd = open(lockpath, O_CREAT | O_RDONLY, 0600);

	if (lfd < 0) {
		err("open %s: %s", lockpath, strerror(errno));
		return -1;
	}

	if (flock(lfd, LOCK_EX | LOCK_NB))
		return -1;

	unlink(SOCKET_PATH);
	if (bind(sd, (struct sockaddr *) &addr, sizeof addr) < 0) {
		err("bind %s: %s", SOCKET_PATH, strerror(errno));
		return -1;
	}

	if (listen(sd, 20) < 0) {
		err("listen: %s", strerror(errno));
		return -1;
	}

	chmod(SOCKET_PATH, 0660);

	return sd;
}
