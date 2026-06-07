/*
 * keyd - A key remapping daemon.
 *
 * NSEvent-based NX_SYSDEFINED bridge for macOS special/media keys.
 *
 * CGEvent has no public field numbers for the compound-event data used by
 * NX_SYSDEFINED media-key events; every accessible field constant
 * (kCGMouseEventSubtype, kCGTabletEventTiltX, etc.) belongs to a different
 * event class.  NSEvent is the only supported API for constructing and
 * decoding these events correctly — it maps subtype/data1/data2 through
 * private internal fields that are not exposed in CGEventTypes.h.
 *
 * Link with: -framework AppKit
 */

#import <AppKit/NSEvent.h>
#include <CoreGraphics/CoreGraphics.h>
#include <IOKit/hidsystem/IOLLEvent.h>

#include "special_keys.h"

#define KEYD_EVENT_MARKER ((int64_t)0x6B657964ULL)

void macos_post_special_key(uint16_t keytype, int pressed)
{
    @autoreleasepool {
        NSInteger data1 = ((NSInteger)keytype << 16) |
                          ((pressed ? 0x0A : 0x0B) << 8);

        NSEvent *nse = [NSEvent
            otherEventWithType:NSEventTypeSystemDefined
                      location:NSZeroPoint
                 modifierFlags:0
                     timestamp:0
                  windowNumber:0
                       context:nil
                       subtype:NX_SUBTYPE_AUX_CONTROL_BUTTONS
                         data1:data1
                         data2:-1];
        CGEventRef ev = nse.CGEvent;
        if (!ev)
            return;
        CGEventSetIntegerValueField(ev, kCGEventSourceUserData,
                                    KEYD_EVENT_MARKER);
        CGEventPost(kCGHIDEventTap, ev);
    }
}

int macos_decode_special_key(CGEventRef event,
                              uint16_t  *keytype_out,
                              int       *pressed_out)
{
    @autoreleasepool {
        NSEvent *nse = [NSEvent eventWithCGEvent:event];
        if (!nse || nse.subtype != NX_SUBTYPE_AUX_CONTROL_BUTTONS)
            return 0;
        int32_t data1  = (int32_t)nse.data1;
        *keytype_out   = (uint16_t)((uint32_t)data1 >> 16);
        *pressed_out   = ((data1 >> 8) & 0xFF) == 0x0A ? 1 : 0;
        return 1;
    }
}
