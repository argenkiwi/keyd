/*
 * keyd - A key remapping daemon.
 *
 * © 2019 Raheman Vaiya (see also: LICENSE).
 */

#include "../../keyd.h"
#include "../../platform.h"

#include <mach/mach_time.h>
#include <IOKit/hid/IOHIDManager.h>
#include <CoreFoundation/CoreFoundation.h>
#include <ApplicationServices/ApplicationServices.h>
#include <mach/thread_policy.h>
#include <mach/thread_act.h>
#include <sys/mman.h>

static uint8_t usage_to_keyd[256] = {
	[0x04] = KEYD_A, [0x05] = KEYD_B, [0x06] = KEYD_C, [0x07] = KEYD_D,
	[0x08] = KEYD_E, [0x09] = KEYD_F, [0x0A] = KEYD_G, [0x0B] = KEYD_H,
	[0x0C] = KEYD_I, [0x0D] = KEYD_J, [0x0E] = KEYD_K, [0x0F] = KEYD_L,
	[0x10] = KEYD_M, [0x11] = KEYD_N, [0x12] = KEYD_O, [0x13] = KEYD_P,
	[0x14] = KEYD_Q, [0x15] = KEYD_R, [0x16] = KEYD_S, [0x17] = KEYD_T,
	[0x18] = KEYD_U, [0x19] = KEYD_V, [0x1A] = KEYD_W, [0x1B] = KEYD_X,
	[0x1C] = KEYD_Y, [0x1D] = KEYD_Z,
	[0x1E] = KEYD_1, [0x1F] = KEYD_2, [0x20] = KEYD_3, [0x21] = KEYD_4,
	[0x22] = KEYD_5, [0x23] = KEYD_6, [0x24] = KEYD_7, [0x25] = KEYD_8,
	[0x26] = KEYD_9, [0x27] = KEYD_0,
	[0x28] = KEYD_ENTER, [0x29] = KEYD_ESC, [0x2A] = KEYD_BACKSPACE, [0x2B] = KEYD_TAB, [0x2C] = KEYD_SPACE,
	[0x2D] = KEYD_MINUS, [0x2E] = KEYD_EQUAL, [0x2F] = KEYD_LEFTBRACE, [0x30] = KEYD_RIGHTBRACE, [0x31] = KEYD_BACKSLASH,
	[0x33] = KEYD_SEMICOLON, [0x34] = KEYD_APOSTROPHE, [0x35] = KEYD_GRAVE, [0x36] = KEYD_COMMA, [0x37] = KEYD_DOT, [0x38] = KEYD_SLASH,
	[0x39] = KEYD_CAPSLOCK,
	[0x3A] = KEYD_F1, [0x3B] = KEYD_F2, [0x3C] = KEYD_F3, [0x3D] = KEYD_F4, [0x3E] = KEYD_F5, [0x3F] = KEYD_F6,
	[0x40] = KEYD_F7, [0x41] = KEYD_F8, [0x42] = KEYD_F9, [0x43] = KEYD_F10, [0x44] = KEYD_F11, [0x45] = KEYD_F12,
	[0x4F] = KEYD_RIGHT, [0x50] = KEYD_LEFT, [0x51] = KEYD_DOWN, [0x52] = KEYD_UP,
	[0xE0] = KEYD_LEFTCTRL, [0xE1] = KEYD_LEFTSHIFT, [0xE2] = KEYD_LEFTALT, [0xE3] = KEYD_LEFTMETA,
	[0xE4] = KEYD_RIGHTCTRL, [0xE5] = KEYD_RIGHTSHIFT, [0xE6] = KEYD_RIGHTALT, [0xE7] = KEYD_RIGHTMETA,
};

static CGKeyCode keyd_to_macos[256] = {
	[KEYD_A] = 0, [KEYD_B] = 11, [KEYD_C] = 8, [KEYD_D] = 2,
	[KEYD_E] = 14, [KEYD_F] = 3, [KEYD_G] = 5, [KEYD_H] = 4,
	[KEYD_I] = 34, [KEYD_J] = 38, [KEYD_K] = 40, [KEYD_L] = 37,
	[KEYD_M] = 46, [KEYD_N] = 45, [KEYD_O] = 31, [KEYD_P] = 35,
	[KEYD_Q] = 12, [KEYD_R] = 15, [KEYD_S] = 1, [KEYD_T] = 17,
	[KEYD_U] = 32, [KEYD_V] = 9, [KEYD_W] = 13, [KEYD_X] = 7,
	[KEYD_Y] = 16, [KEYD_Z] = 6,
	[KEYD_1] = 18, [KEYD_2] = 19, [KEYD_3] = 20, [KEYD_4] = 21,
	[KEYD_5] = 23, [KEYD_6] = 22, [KEYD_7] = 26, [KEYD_8] = 28,
	[KEYD_9] = 25, [KEYD_0] = 29,
	[KEYD_ENTER] = 36, [KEYD_ESC] = 53, [KEYD_BACKSPACE] = 51, [KEYD_TAB] = 48, [KEYD_SPACE] = 49,
	[KEYD_LEFTCTRL] = 59, [KEYD_LEFTSHIFT] = 56, [KEYD_LEFTALT] = 58, [KEYD_LEFTMETA] = 55,
	[KEYD_RIGHTCTRL] = 62, [KEYD_RIGHTSHIFT] = 60, [KEYD_RIGHTALT] = 61, [KEYD_RIGHTMETA] = 54,
};

static long macos_get_time_ms(void)
{
	static mach_timebase_info_data_t tb;
	if (tb.denom == 0) mach_timebase_info(&tb);
	uint64_t time = mach_absolute_time();
	return (time * tb.numer / tb.denom) / 1000000;
}

static void macos_set_realtime(void)
{
	thread_time_constraint_policy_data_t policy;
	policy.period = 100000;
	policy.computation = 50000;
	policy.constraint = 85000;
	policy.preemptible = 1;

	thread_policy_set(mach_thread_self(), THREAD_TIME_CONSTRAINT_POLICY,
			 (thread_policy_t)&policy, THREAD_TIME_CONSTRAINT_POLICY_COUNT);
}

static void macos_lock_memory(void)
{
	if (mlockall(MCL_CURRENT | MCL_FUTURE)) {
		perror("mlockall");
	}
}

static IOHIDManagerRef hid_manager;

static int macos_device_scan(struct device *devices)
{
	if (!hid_manager) {
		hid_manager = IOHIDManagerCreate(kCFAllocatorDefault, kIOHIDOptionsTypeNone);
		IOHIDManagerSetDeviceMatching(hid_manager, NULL);
		IOHIDManagerOpen(hid_manager, kIOHIDOptionsTypeNone);
	}

	CFSetRef device_set = IOHIDManagerCopyDevices(hid_manager);
	if (!device_set) return 0;

	CFIndex count = CFSetGetCount(device_set);
	IOHIDDeviceRef *device_refs = malloc(sizeof(IOHIDDeviceRef) * count);
	CFSetGetValues(device_set, (const void **)device_refs);

	int n = 0;
	for (CFIndex i = 0; i < count && n < MAX_DEVICES; i++) {
		IOHIDDeviceRef dev_ref = device_refs[i];
		struct device *dev = &devices[n];

		memset(dev, 0, sizeof(*dev));
		dev->handle = dev_ref;

		CFStringRef name = IOHIDDeviceGetProperty(dev_ref, CFSTR(kIOHIDProductKey));
		if (name) {
			CFStringGetCString(name, dev->name, sizeof(dev->name), kCFStringEncodingUTF8);
		}

		long vendor_id = 0, product_id = 0;
		CFNumberRef vid_ref = IOHIDDeviceGetProperty(dev_ref, CFSTR(kIOHIDVendorIDKey));
		CFNumberRef pid_ref = IOHIDDeviceGetProperty(dev_ref, CFSTR(kIOHIDProductIDKey));
		if (vid_ref) CFNumberGetValue(vid_ref, kCFNumberLongType, &vendor_id);
		if (pid_ref) CFNumberGetValue(pid_ref, kCFNumberLongType, &product_id);

		snprintf(dev->id, sizeof(dev->id), "%04lx:%04lx", vendor_id, product_id);

		// Determine capabilities
		if (IOHIDDeviceConformsTo(dev_ref, kHIDPage_GenericDesktop, kHIDUsage_GD_Keyboard)) {
			dev->capabilities |= CAP_KEYBOARD | CAP_KEY;
		}
		if (IOHIDDeviceConformsTo(dev_ref, kHIDPage_GenericDesktop, kHIDUsage_GD_Mouse)) {
			dev->capabilities |= CAP_MOUSE;
		}

		n++;
	}

	free(device_refs);
	CFRelease(device_set);
	return n;
}

static int macos_device_grab(struct device *dev)
{
	IOReturn res = IOHIDDeviceOpen(dev->handle, kIOHIDOptionsTypeSeizeDevice);
	if (res == kIOReturnSuccess) {
		dev->grabbed = 1;
		return 0;
	}
	return -1;
}

static int macos_device_ungrab(struct device *dev)
{
	IOReturn res = IOHIDDeviceClose(dev->handle, kIOHIDOptionsTypeNone);
	if (res == kIOReturnSuccess) {
		dev->grabbed = 0;
		return 0;
	}
	return -1;
}

struct macos_event_node {
	struct device_event event;
	struct macos_event_node *next;
};

static void macos_device_set_led(const struct device *dev, int led, int state)
{
	uint8_t report = 0;
	if (led == 1) report = state ? 0x02 : 0;
	IOHIDDeviceSetReport(dev->handle, kIOHIDReportTypeOutput, 0, &report, sizeof(report));
}

static struct device_event *macos_device_read_event(struct device *dev)
{
	struct macos_event_node *node = dev->platform_data;
	if (!node) return NULL;

	static struct device_event devev;
	devev = node->event;

	dev->platform_data = node->next;
	free(node);

	return &devev;
}

static int aux_fds[32];
static int nr_aux_fds = 0;
static int current_timeout = 0;
static int (*current_event_handler)(struct event *ev);

static void macos_evloop_add_fd(int fd)
{
	if (nr_aux_fds < 32)
		aux_fds[nr_aux_fds++] = fd;
}

static void fd_callback(CFFileDescriptorRef fdref, CFOptionFlags callBackTypes, void *info)
{
	int fd = (int)(long)info;
	(void)callBackTypes;
	struct event ev = {
		.type = EV_FD_ACTIVITY,
		.fd = fd,
		.timestamp = macos_get_time_ms()
	};

	current_timeout = current_event_handler(&ev);
	CFFileDescriptorEnableCallBacks(fdref, kCFFileDescriptorReadCallBack);
}

static void hid_input_callback(void *context, IOReturn result, void *sender, IOHIDValueRef value)
{
	(void)context;
	(void)result;
	(void)sender;

	IOHIDElementRef element = IOHIDValueGetElement(value);
	IOHIDDeviceRef device_ref = IOHIDElementGetDevice(element);
	
	uint32_t usagePage = IOHIDElementGetUsagePage(element);
	uint32_t usage = IOHIDElementGetUsage(element);
	long val = IOHIDValueGetIntegerValue(value);

	if (usagePage != kHIDPage_KeyboardOrKeypad)
		return;

	struct device *dev = NULL;
	for (size_t i = 0; i < device_table_sz; i++) {
		if (device_table[i].handle == device_ref) {
			dev = &device_table[i];
			break;
		}
	}

	if (!dev) return;

	uint8_t code = usage_to_keyd[usage < 256 ? usage : 0];
	if (!code) return;

	struct macos_event_node *node = calloc(1, sizeof(struct macos_event_node));
	node->event.type = DEV_KEY;
	node->event.code = code;
	node->event.pressed = val != 0;

	// Append to queue
	struct macos_event_node **curr = (struct macos_event_node **)&dev->platform_data;
	while (*curr) curr = &(*curr)->next;
	*curr = node;

	struct event ev = {
		.type = EV_DEV_EVENT,
		.dev = dev,
		.devev = &node->event,
		.timestamp = macos_get_time_ms()
	};

	current_timeout = current_event_handler(&ev);
}

static int macos_evloop(int (*event_handler) (struct event *ev))
{
	current_event_handler = event_handler;

	device_table_sz = macos_device_scan(device_table);
	for (size_t i = 0; i < device_table_sz; i++) {
		struct event ev = {
			.type = EV_DEV_ADD,
			.dev = &device_table[i],
			.timestamp = macos_get_time_ms()
		};
		event_handler(&ev);
	}

	if (hid_manager) {
		IOHIDManagerRegisterInputValueCallback(hid_manager, hid_input_callback, NULL);
		IOHIDManagerScheduleWithRunLoop(hid_manager, CFRunLoopGetCurrent(), kCFRunLoopDefaultMode);
	}

	for (int i = 0; i < nr_aux_fds; i++) {
		CFFileDescriptorContext ctx = { .version = 0, .info = (void *)(long)aux_fds[i] };
		CFFileDescriptorRef fdref = CFFileDescriptorCreate(kCFAllocatorDefault, aux_fds[i], false, fd_callback, &ctx);
		CFFileDescriptorEnableCallBacks(fdref, kCFFileDescriptorReadCallBack);
		CFRunLoopSourceRef source = CFFileDescriptorCreateRunLoopSource(kCFAllocatorDefault, fdref, 0);
		CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopDefaultMode);
		CFRelease(source);
	}

	while (1) {
		CFRunLoopRunInMode(kCFRunLoopDefaultMode, current_timeout > 0 ? current_timeout / 1000.0 : 1.0, false);
		
		struct event ev = {
			.type = EV_TIMEOUT,
			.timestamp = macos_get_time_ms()
		};
		current_timeout = event_handler(&ev);
	}

	return 0;
}

struct vkbd {
	CGEventSourceRef source;
};

static struct vkbd *macos_vkbd_init(const char *name)
{
	struct vkbd *vkbd = calloc(1, sizeof(struct vkbd));
	vkbd->source = CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
	return vkbd;
}

static void macos_vkbd_send_key(const struct vkbd *vkbd, uint8_t code, int state)
{
	CGKeyCode macos_code = keyd_to_macos[code];
	CGEventRef ev = CGEventCreateKeyboardEvent(vkbd->source, macos_code, state != 0);
	CGEventPost(kCGHIDEventTap, ev);
	CFRelease(ev);
}

static void macos_vkbd_mouse_move(const struct vkbd *vkbd, int x, int y) {}
static void macos_vkbd_mouse_move_abs(const struct vkbd *vkbd, int x, int y) {}
static void macos_vkbd_mouse_scroll(const struct vkbd *vkbd, int x, int y) {}

static void macos_free_vkbd(struct vkbd *vkbd)
{
	if (vkbd) {
		CFRelease(vkbd->source);
		free(vkbd);
	}
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
	.vkbd_send_key = macos_vkbd_send_key,
	.vkbd_mouse_move = macos_vkbd_mouse_move,
	.vkbd_mouse_move_abs = macos_vkbd_mouse_move_abs,
	.vkbd_mouse_scroll = macos_vkbd_mouse_scroll,
	.free_vkbd = macos_free_vkbd,

	.ipc_create_server = macos_ipc_create_server,
	.ipc_connect = macos_ipc_connect,
};

const struct platform *platform = &macos_platform;
