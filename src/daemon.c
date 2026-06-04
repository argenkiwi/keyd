#include "keyd.h"

struct config_ent {
	struct config config;
	struct keyboard *kbd;
	struct config_ent *next;
};

struct daemon {
	int ipcfd;
	struct vkbd *vkbd;
	struct config_ent *configs;
	uint8_t keystate[256];
	int listeners[32];
	size_t nr_listeners;
	struct keyboard *active_kbd;
};

static struct daemon *g_daemon_ptr;

static void free_configs(struct daemon *d)
{
	struct config_ent *ent = d->configs;
	while (ent) {
		struct config_ent *tmp = ent;
		ent = ent->next;
		free(tmp->kbd);
		free(tmp);
	}

	d->configs = NULL;
}

static void cleanup(void)
{
	free_configs(g_daemon_ptr);
	free_vkbd(g_daemon_ptr->vkbd);
}

static void clear_vkbd(struct daemon *d)
{
	size_t i;

	for (i = 0; i < 256; i++)
		if (d->keystate[i]) {
			vkbd_send_key(d->vkbd, i, 0);
			d->keystate[i] = 0;
		}
}

static void send_key(void *ctx, uint8_t code, uint8_t state)
{
	struct daemon *d = ctx;

	d->keystate[code] = state;

	switch (code) {
		case KEYD_SCROLL_DOWN:
			if (state)
				vkbd_mouse_scroll(d->vkbd, 0, -1);
			break;
		case KEYD_SCROLL_UP:
			if (state)
				vkbd_mouse_scroll(d->vkbd, 0, 1);
			break;
		case KEYD_SCROLL_RIGHT:
			if (state)
				vkbd_mouse_scroll(d->vkbd, 1, 0);
			break;
		case KEYD_SCROLL_LEFT:
			if (state)
				vkbd_mouse_scroll(d->vkbd, -1, 0);
			break;
		default:
			vkbd_send_key(d->vkbd, code, state);
			break;
	}
}

static void send_key_macro_wrapper(void *ctx, uint8_t code, uint8_t state)
{
	send_key(ctx, code, state);
}

static void add_listener(struct daemon *d, int con)
{
	struct timeval tv;

	/*
	 * In order to avoid blocking the main event loop, allow up to 50ms for
	 * slow clients to relieve back pressure before dropping them.
	 */
	tv.tv_usec = 50000;
	tv.tv_sec = 0;

	if (d->nr_listeners == ARRAY_SIZE(d->listeners)) {
		char s[] = "Max listeners exceeded\n";
		xwrite(con, &s, sizeof s);

		close(con);
		return;
	}

	setsockopt(con, SOL_SOCKET, SO_SNDTIMEO, &tv, sizeof tv);

	if (d->active_kbd) {
		size_t i;
		struct config *config = &d->active_kbd->config;
		struct layer *layout = &config->layers[0];

		for (i = 1; i < config->nr_layers; i++)
			if (d->active_kbd->layer_state[i].active) {
				struct layer *layer = &config->layers[i];

				if (layer->type == LT_LAYOUT) {
					layout = layer;
					break;
				}
			}

		dprintf(con, "/%s\n", layout->name);

		for (i = 1; i < config->nr_layers; i++) {
			if (d->active_kbd->layer_state[i].active) {
				ssize_t ret;
				struct layer *layer = &config->layers[i];

				if (layer->type != LT_LAYOUT) {
					ret = dprintf(con, "+%s\n", layer->name);
					if (ret < 0)
						goto fail;
				}
			}
		}
	}

	d->listeners[d->nr_listeners++] = con;
	return;
fail:
	close(con);
	return;
}

static void on_layer_change(void *ctx, const struct keyboard *kbd, const struct layer *layer, uint8_t state)
{
	struct daemon *d = ctx;
	size_t i;
	char buf[MAX_LAYER_NAME_LEN+2];
	ssize_t bufsz;

	int keep[ARRAY_SIZE(d->listeners)];
	size_t n = 0;

	if (kbd->config.layer_indicator) {
		int active_layers = 0;

		for (i = 1; i < kbd->config.nr_layers; i++)
			if (kbd->config.layers[i].type != LT_LAYOUT && kbd->layer_state[i].active) {
				active_layers = 1;
				break;
			}

		for (i = 0; i < device_table_sz; i++)
			if (device_table[i].kbd == kbd)
				device_set_led(&device_table[i], 1, active_layers);
	}

	if (!d->nr_listeners)
		return;

	if (layer->type == LT_LAYOUT)
		bufsz = snprintf(buf, sizeof(buf), "/%s\n", layer->name);
	else
		bufsz = snprintf(buf, sizeof(buf), "%c%s\n", state ? '+' : '-', layer->name);

	for (i = 0; i < d->nr_listeners; i++) {
		ssize_t nw = write(d->listeners[i], buf, bufsz);

		if (nw == bufsz)
			keep[n++] = d->listeners[i];
		else
			close(d->listeners[i]);
	}

	if (n != d->nr_listeners) {
		d->nr_listeners = n;
		memcpy(d->listeners, keep, n * sizeof(int));
	}
}

static int load_configs(struct daemon *d)
{
	DIR *dh = opendir(CONFIG_DIR);
	struct dirent *dirent;

	if (!dh) {
		err("opendir %s: %s", CONFIG_DIR, strerror(errno));
		return -1;
	}

	d->configs = NULL;

	while ((dirent = readdir(dh))) {
		char path[1024];
		int len;

		if (dirent->d_type == DT_DIR)
			continue;

		len = snprintf(path, sizeof path, "%s/%s", CONFIG_DIR, dirent->d_name);

		if (len >= 5 && !strcmp(path + len - 5, ".conf")) {
			struct config_ent *ent = calloc(1, sizeof(struct config_ent));

			keyd_log("CONFIG: parsing b{%s}\n", path);

			if (config_parse(&ent->config, path) >= 0) {
				struct output output = {
					.ctx = d,
					.send_key = send_key,
					.on_layer_change = on_layer_change,
				};
				ent->kbd = new_keyboard(&ent->config, &output);

				ent->next = d->configs;
				d->configs = ent;
			} else {
				free(ent);
				keyd_log("DEVICE: y{WARNING} failed to parse %s\n", path);
			}

		}
	}

	closedir(dh);
	return 0;
}

static struct config_ent *lookup_config_ent(struct daemon *d, const char *id, uint8_t flags)
{
	struct config_ent *ent = d->configs;
	struct config_ent *match = NULL;
	int rank = 0;

	while (ent) {
		int r = config_check_match(&ent->config, id, flags);

		if (r > rank) {
			match = ent;
			rank = r;
		}

		ent = ent->next;
	}

	/* The wildcard should not match mice or trackpads. */
	if (rank == 1 && !((flags & ID_KEYBOARD) && !(flags & ID_TRACKPAD))) {
		return NULL;
	} else {
		if (flags & ID_TRACKPAD)
			keyd_log("y{WARNING}: %s appears to be a trackpad, which is current unsupported. Mouse movement is likely to break(YMMV)\n", id);

		return match;
	}
}

static void manage_device(struct daemon *d, struct device *dev)
{
	uint8_t flags = 0;
	struct config_ent *ent;

	if (dev->is_virtual)
		return;

	if (dev->capabilities & CAP_KEY)
		flags |= ID_KEY;
	if (dev->capabilities & CAP_KEYBOARD)
		flags |= ID_KEYBOARD;
	if (dev->capabilities & CAP_MOUSE_ABS)
		flags |= ID_TRACKPAD;
	if (dev->capabilities & CAP_MOUSE)
		flags |= ID_MOUSE;

	if ((ent = lookup_config_ent(d, dev->id, flags))) {
		if (device_grab(dev)) {
			keyd_log("DEVICE: y{WARNING} Failed to grab %s\n", dev->path);
			dev->kbd = NULL;
			return;
		}

		keyd_log("DEVICE: g{match}    %s  %s\t(%s)\n",
			  dev->id, ent->config.path, dev->name);

		dev->kbd = ent->kbd;
	} else {
		dev->kbd = NULL;
		device_ungrab(dev);
		keyd_log("DEVICE: r{ignoring} %s  (%s)\n", dev->id, dev->name);
	}
}

static void reload(struct daemon *d)
{
	size_t i;

	free_configs(d);
	if (load_configs(d) < 0) {
		keyd_log("DAEMON: y{WARNING} failed to load configs: %s\n", errstr);
		return;
	}

	for (i = 0; i < device_table_sz; i++)
		manage_device(d, &device_table[i]);

	clear_vkbd(d);
}

static void send_success(int con)
{
	struct ipc_message msg = {0};

	msg.type = IPC_SUCCESS;;
	msg.sz = 0;

	xwrite(con, &msg, sizeof msg);
	close(con);
}

static void send_fail(int con, const char *fmt, ...)
{
	struct ipc_message msg = {0};
	va_list args;

	va_start(args, fmt);

	msg.type = IPC_FAIL;
	msg.sz = vsnprintf(msg.data, sizeof(msg.data), fmt, args);

	xwrite(con, &msg, sizeof msg);
	close(con);

	va_end(args);
}

static int input(struct daemon *d, char *buf, size_t sz, uint32_t timeout)
{
	size_t i;
	uint32_t codepoint;
	(void)sz;
	uint8_t codes[4];

	int csz;

	while ((csz = utf8_read_char(buf, &codepoint))) {
		int found = 0;
		char s[2];

		if (csz == 1) {
			uint8_t code, mods;
			s[0] = (char)codepoint;
			s[1] = 0;

			found = 1;
			if (!parse_key_sequence(s, &code, &mods)) {
				if (mods & MOD_SHIFT) {
					vkbd_send_key(d->vkbd, KEYD_LEFTSHIFT, 1);
					vkbd_send_key(d->vkbd, code, 1);
					vkbd_send_key(d->vkbd, code, 0);
					vkbd_send_key(d->vkbd, KEYD_LEFTSHIFT, 0);
				} else {
					vkbd_send_key(d->vkbd, code, 1);
					vkbd_send_key(d->vkbd, code, 0);
				}
			} else if ((char)codepoint == ' ') {
				vkbd_send_key(d->vkbd, KEYD_SPACE, 1);
				vkbd_send_key(d->vkbd, KEYD_SPACE, 0);
			} else if ((char)codepoint == '\n') {
				vkbd_send_key(d->vkbd, KEYD_ENTER, 1);
				vkbd_send_key(d->vkbd, KEYD_ENTER, 0);
			} else if ((char)codepoint == '\t') {
				vkbd_send_key(d->vkbd, KEYD_TAB, 1);
				vkbd_send_key(d->vkbd, KEYD_TAB, 0);
			} else {
				found = 0;
			}
		}

		if (!found) {
			int idx = unicode_lookup_index(codepoint);
			if (idx < 0) {
				err("ERROR: could not find code for \"%.*s\"", csz, buf);
				return -1;
			}

			unicode_get_sequence(idx, codes);

			for (i = 0; i < 4; i++) {
				vkbd_send_key(d->vkbd, codes[i], 1);
				vkbd_send_key(d->vkbd, codes[i], 0);
			}
		}
		buf+=csz;

		if (timeout)
			usleep(timeout);
	}

	return 0;
}

static void handle_client(struct daemon *d, int con)
{
	struct ipc_message msg;

	xread(con, &msg, sizeof msg);

	if (msg.sz >= sizeof(msg.data)) {
		send_fail(con, "maximum message size exceeded");
		return;
	}
	msg.data[msg.sz] = 0;

	if (msg.timeout > 1000000) {
		send_fail(con, "timeout cannot exceed 1000 ms");
		return;
	}

	switch (msg.type) {
		struct config_ent *ent;
		int success;
		struct macro macro;

	case IPC_MACRO:
		while (msg.sz && msg.data[msg.sz-1] == '\n')
			msg.data[--msg.sz] = 0;

		if (macro_parse(msg.data, &macro)) {
			send_fail(con, "%s", errstr);
			return;
		}

		macro_execute(send_key_macro_wrapper, d, &macro, msg.timeout);
		send_success(con);

		break;
	case IPC_INPUT:
		if (input(d, msg.data, msg.sz, msg.timeout))
			send_fail(con, "%s", errstr);
		else
			send_success(con);
		break;
	case IPC_RELOAD:
		reload(d);
		send_success(con);
		break;
	case IPC_LAYER_LISTEN:
		add_listener(d, con);
		break;
	case IPC_BIND:
		success = 0;

		if (msg.sz == sizeof(msg.data)) {
			send_fail(con, "bind expression size exceeded");
			return;
		}

		msg.data[msg.sz] = 0;

		for (ent = d->configs; ent; ent = ent->next) {
			if (!kbd_eval(ent->kbd, msg.data))
				success = 1;
		}

		if (success)
			send_success(con);
		else
			send_fail(con, "%s", errstr);


		break;
	default:
		send_fail(con, "Unknown command");
		break;
	}
}

static long process_keypress(struct keyboard *kbd, uint8_t code, int timestamp)
{
	struct key_event kev = {
		.code = code,
		.pressed = 1,
		.timestamp = timestamp
	};

	kbd_process_events(kbd, &kev, 1);

	kev.pressed = 0;
	return kbd_process_events(kbd, &kev, 1);
}

static int event_handler(struct event *ev, void *ctx)
{
	static int last_time = 0;
	static int timeout = 0;
	struct daemon *d = ctx;
	struct key_event kev = {0};

	timeout -= ev->timestamp - last_time;
	last_time = ev->timestamp;

	timeout = timeout < 0 ? 0 : timeout;

	switch (ev->type) {
	case EV_TIMEOUT:
		if (!d->active_kbd)
			return 0;

		kev.code = 0;
		kev.timestamp = ev->timestamp;

		timeout = kbd_process_events(d->active_kbd, &kev, 1);
		break;
	case EV_DEV_EVENT:
		if (ev->dev->kbd) {
			struct keyboard *kbd = ev->dev->kbd;
			d->active_kbd = ev->dev->kbd;
			switch (ev->devev->type) {
			size_t i;
			case DEV_KEY:
				dbg("input %s %s", KEY_NAME(ev->devev->code), ev->devev->pressed ? "down" : "up");

				kev.code = ev->devev->code;
				kev.pressed = ev->devev->pressed;
				kev.timestamp = ev->timestamp;

				timeout = kbd_process_events(kbd, &kev, 1);
				break;
			case DEV_MOUSE_MOVE:
				if (kbd->scroll.active) {
					if (kbd->scroll.sensitivity == 0)
						break;
					int xticks, yticks;

					kbd->scroll.y += ev->devev->y;
					kbd->scroll.x += ev->devev->x;

					yticks = kbd->scroll.y / kbd->scroll.sensitivity;
					kbd->scroll.y %= kbd->scroll.sensitivity;

					xticks = kbd->scroll.x / kbd->scroll.sensitivity;
					kbd->scroll.x %= kbd->scroll.sensitivity;

					vkbd_mouse_scroll(d->vkbd, 0, -1*yticks);
					vkbd_mouse_scroll(d->vkbd, 0, xticks);
				} else {
					vkbd_mouse_move(d->vkbd, ev->devev->x, ev->devev->y);
				}
				break;
			case DEV_MOUSE_MOVE_ABS:
				vkbd_mouse_move_abs(d->vkbd, ev->devev->x, ev->devev->y);
				break;
			case DEV_MOUSE_SCROLL:
				if (d->active_kbd) {
					if (ev->devev->x > 0)
						for (i = 0;i < (size_t)ev->devev->x; i++)
							timeout = process_keypress(d->active_kbd, KEYD_SCROLL_RIGHT, ev->timestamp);
					if (ev->devev->x < 0)
						for (i = 0;i < (size_t)-1*ev->devev->x; i++)
							timeout = process_keypress(d->active_kbd, KEYD_SCROLL_LEFT, ev->timestamp);
					if (ev->devev->y > 0)
						for (i = 0;i < (size_t)ev->devev->y; i++)
							timeout = process_keypress(d->active_kbd, KEYD_SCROLL_UP, ev->timestamp);
					if (ev->devev->y < 0)
						for (i = 0;i < (size_t)-1*ev->devev->y; i++)
							timeout = process_keypress(d->active_kbd, KEYD_SCROLL_DOWN, ev->timestamp);
				}
				break;
			default:
				break;
			}
		} else if (!ev->dev->is_virtual && ev->dev->capabilities & CAP_MOUSE) {
			if (d->active_kbd && (ev->devev->type == DEV_KEY || ev->devev->type == DEV_MOUSE_SCROLL))
				timeout = process_keypress(d->active_kbd, KEYD_EXTERNAL_MOUSE_BUTTON, ev->timestamp);
		} else if (ev->dev->is_virtual && ev->devev->type == DEV_LED) {
			size_t i;

			/*
			 * Propagate LED events received by the virtual device from userspace
			 * to all grabbed devices.
			 *
			 * NOTE/TODO: Account for potential layer_indicator interference
			 */
			for (i = 0; i < device_table_sz; i++)
				if (device_table[i].kbd)
					device_set_led(&device_table[i], ev->devev->code, ev->devev->pressed);
		}

		break;
	case EV_DEV_ADD:
		manage_device(d, ev->dev);
		break;
	case EV_DEV_REMOVE:
		keyd_log("DEVICE: r{removed}\t%s %s\n", ev->dev->id, ev->dev->name);

		break;
	case EV_FD_ACTIVITY:
		if (ev->fd == d->ipcfd) {
			int con = accept(d->ipcfd, NULL, 0);
			if (con < 0) {
				perror("accept");
				exit(-1);
			}

			handle_client(d, con);
		}
		break;
	default:
		break;
	}

	return timeout;
}

int run_daemon(int argc, char *argv[])
{
	struct daemon d = {0};
#ifndef __APPLE__
	struct sched_param sp;
#endif

	(void)argc;
	(void)argv;

	d.ipcfd = ipc_create_server();

	if (d.ipcfd < 0)
		die("failed to create %s (another instance already running?)", SOCKET_PATH);

	d.vkbd = vkbd_init(VKBD_NAME);

	setvbuf(stdout, NULL, _IOLBF, 0);
	setvbuf(stderr, NULL, _IOLBF, 0);

#ifndef __APPLE__
	/* Real-time scheduling and memory locking require special entitlements
	 * on macOS and are skipped; they remain on Linux for latency. */
	if (sched_getparam(0, &sp)) {
		perror("sched_getparam");
		exit(-1);
	}

	sp.sched_priority = 49;
	if (sched_setscheduler(0, SCHED_FIFO, &sp)) {
		perror("sched_setscheduler");
		exit(-1);
	}

	if (mlockall(MCL_CURRENT | MCL_FUTURE)) {
		perror("mlockall");
		exit(-1);
	}
#endif /* !__APPLE__ */

	evloop_add_fd(d.ipcfd);

	g_daemon_ptr = &d;
	atexit(cleanup);

	reload(&d);

	keyd_log("Starting keyd "VERSION"\n");
	evloop(event_handler, &d);

	return 0;
}
