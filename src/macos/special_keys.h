/*
 * keyd - A key remapping daemon.
 *
 * macOS special-key (NX_SYSDEFINED) event bridge.
 *
 * CGEvent has no public field constants for the compound-event data fields
 * used by NX_SYSDEFINED media-key events.  NSEvent is the only supported
 * way to construct and decode them.  This header exposes a plain-C API
 * implemented in special_keys.m (Objective-C) so that the rest of the
 * codebase can stay in C.
 */
#ifndef MACOS_SPECIAL_KEYS_H
#define MACOS_SPECIAL_KEYS_H

#include <stdint.h>
#include <CoreGraphics/CoreGraphics.h>

/* Post an NX_SYSDEFINED media/special key event via NSEvent. */
void macos_post_special_key(uint16_t keytype, int pressed);

/*
 * Decode an NX_SYSDEFINED CGEvent into its keytype and direction.
 * Returns 1 if the event is NX_SUBTYPE_AUX_CONTROL_BUTTONS, 0 otherwise.
 */
int macos_decode_special_key(CGEventRef event,
                              uint16_t  *keytype_out,
                              int       *pressed_out);

#endif
