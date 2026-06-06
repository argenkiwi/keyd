#include "../../keyd.h"

void vkbd_send_key(const struct vkbd *vkbd, uint8_t code, int state)
{
	platform->vkbd_send_key(vkbd, code, state);
}

void vkbd_mouse_move(const struct vkbd *vkbd, int x, int y)
{
	platform->vkbd_mouse_move(vkbd, x, y);
}

void vkbd_mouse_move_abs(const struct vkbd *vkbd, int x, int y)
{
	platform->vkbd_mouse_move_abs(vkbd, x, y);
}

void vkbd_mouse_scroll(const struct vkbd *vkbd, int x, int y)
{
	platform->vkbd_mouse_scroll(vkbd, x, y);
}

void free_vkbd(struct vkbd *vkbd)
{
	platform->free_vkbd(vkbd);
}
