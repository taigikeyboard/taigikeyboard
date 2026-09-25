/*
 * taigikeyboard.h — the C ABI of `taigi-linux-ffi`, what the Fcitx5 addon
 * (linux/fcitx5/) links. Implemented, declaration by declaration in this
 * order, by linux/crates/taigi-linux-ffi/src/lib.rs.
 *
 * Contract:
 *   - Handles are opaque and single-owner: what a `*_new` returns is freed
 *     exactly once by the matching `*_free`; null is accepted everywhere and
 *     ignored.
 *   - One runtime per process; one engine per input context; every engine is
 *     freed before its runtime.
 *   - Every `taigi_engine_*` call answers a reply the caller reads through the
 *     `taigi_reply_*` accessors and frees with `taigi_reply_free`. Strings the
 *     accessors return are UTF-8, NUL-terminated, owned by the reply, and valid
 *     until it is freed. Out-of-range indices answer 0 / false / "".
 *   - Calls are not thread-safe: one thread drives one engine at a time (the
 *     framework's event loop).
 *   - A panic inside the library never crosses this boundary: the call answers
 *     null / false / 0 and logs.
 */
#ifndef TAIGIKEYBOARD_H
#define TAIGIKEYBOARD_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct TaigiRuntime TaigiRuntime;
typedef struct TaigiEngine TaigiEngine;
typedef struct TaigiReply TaigiReply;
typedef struct TaigiMenu TaigiMenu;

/* What one reply entry asks the shell to do (`taigi_reply_kind`). */
enum {
    TAIGI_EMIT_PREEDIT = 1,            /* text + caret (characters); shown, underlined */
    TAIGI_EMIT_CLEAR_PREEDIT = 2,      /* the preedit goes away without a commit */
    TAIGI_EMIT_COMMIT = 3,             /* text written into the document */
    TAIGI_EMIT_DELETE_SURROUNDING = 4, /* delete_offset / delete_count, before the caret */
    TAIGI_EMIT_LOOKUP_TABLE = 5,       /* the candidate list (table accessors) */
    TAIGI_EMIT_HIDE_LOOKUP_TABLE = 6,  /* the candidate list goes away */
    TAIGI_EMIT_MODE_CHANGED = 7,       /* the mode changed: re-read taigi_runtime_mode_label / _symbol */
    TAIGI_EMIT_ANNOUNCE_MODE = 8       /* show the mode briefly (showInputMethodInformation) */
};

/* `taigi_engine_navigate` directions. */
enum {
    TAIGI_NAVIGATE_PREVIOUS = 0,
    TAIGI_NAVIGATE_NEXT = 1,
    TAIGI_NAVIGATE_PAGE_UP = 2,
    TAIGI_NAVIGATE_PAGE_DOWN = 3
};

/* `taigi_engine_set_capabilities`: the client can take DELETE_SURROUNDING
 * (the auto-space swap). Same bit as IBus's IBUS_CAP_SURROUNDING_TEXT. */
#define TAIGI_CAP_SURROUNDING_TEXT (1u << 5)

/* Key modifier bits for `taigi_engine_key`'s `states` — the IBus / X11 mask
 * (Fcitx5's KeyState uses the same bits; add TAIGI_STATE_RELEASE yourself for
 * a release, Fcitx5 keeps that out of the mask). */
#define TAIGI_STATE_SHIFT (1u << 0)
#define TAIGI_STATE_LOCK (1u << 1)
#define TAIGI_STATE_CONTROL (1u << 2)
#define TAIGI_STATE_MOD1 (1u << 3)
#define TAIGI_STATE_MOD4 (1u << 6)
#define TAIGI_STATE_SUPER (1u << 26)
#define TAIGI_STATE_RELEASE (1u << 30)

/* The library version ("3.6.9"); static, never freed. */
const char *taigi_version(void);

TaigiRuntime *taigi_runtime_new(void);
void taigi_runtime_free(TaigiRuntime *runtime);
/* The mode label beside the icon ("台羅 · 漢字優先"); freed with
 * taigi_string_free. Null on a panic. */
char *taigi_runtime_mode_label(const TaigiRuntime *runtime);
/* The short form for a tray / panel indicator ("台羅" / "白話") — the
 * symbol the IBus shell shows; freed with taigi_string_free. Null on a
 * panic. */
char *taigi_runtime_mode_symbol(const TaigiRuntime *runtime);
/* Opens the settings window where the user left it (the framework's
 * configure button). false when the binary could not be started. */
bool taigi_open_settings(void);
void taigi_string_free(char *text);
/* The panel menu rows in the display language of the moment: separators
 * and actions with an id (for taigi_engine_menu_activate), a title and a
 * detail (the chord, or ""). Freed with taigi_menu_free. */
TaigiMenu *taigi_runtime_menu(const TaigiRuntime *runtime);
size_t taigi_menu_count(const TaigiMenu *menu);
bool taigi_menu_is_separator(const TaigiMenu *menu, size_t index);
const char *taigi_menu_id(const TaigiMenu *menu, size_t index);
const char *taigi_menu_title(const TaigiMenu *menu, size_t index);
const char *taigi_menu_detail(const TaigiMenu *menu, size_t index);
void taigi_menu_free(TaigiMenu *menu);

TaigiEngine *taigi_engine_new(const TaigiRuntime *runtime);
void taigi_engine_free(TaigiEngine *engine);
void taigi_engine_set_capabilities(TaigiEngine *engine, uint32_t caps);
/* The focused field hides what is typed (CapabilityFlag::Password): keys
 * pass through uncomposed, nothing is learned. */
void taigi_engine_set_password_field(TaigiEngine *engine, bool is_password);

/* One key press or release: X11 keysym, hardware keycode (X keycode = evdev
 * + 8), modifier mask. `taigi_reply_handled` says whether the key was
 * consumed; the entries say what to render either way. */
TaigiReply *taigi_engine_key(TaigiEngine *engine, uint32_t keysym, uint32_t keycode, uint32_t states);
/* Focus loss / reset / switch away: what was on screen is already written
 * (by the framework on focus loss, the client on reset, the shell on a
 * switch); the engine forgets the composition. */
TaigiReply *taigi_engine_end_session(TaigiEngine *engine);
/* A page / cursor step from the panel (TAIGI_NAVIGATE_*). */
TaigiReply *taigi_engine_navigate(TaigiEngine *engine, uint32_t direction);
/* A click on the `position`-th cell of the current page: highlights, never
 * commits. */
TaigiReply *taigi_engine_click(TaigiEngine *engine, uint32_t position);
/* A menu row (taigi_menu_id) was activated for this context. */
TaigiReply *taigi_engine_menu_activate(TaigiEngine *engine, const char *id);

bool taigi_reply_handled(const TaigiReply *reply);
size_t taigi_reply_count(const TaigiReply *reply);
uint32_t taigi_reply_kind(const TaigiReply *reply, size_t index);
const char *taigi_reply_text(const TaigiReply *reply, size_t index);
uint32_t taigi_reply_caret(const TaigiReply *reply, size_t index);
int32_t taigi_reply_delete_offset(const TaigiReply *reply, size_t index);
uint32_t taigi_reply_delete_count(const TaigiReply *reply, size_t index);
size_t taigi_reply_table_count(const TaigiReply *reply, size_t index);
const char *taigi_reply_table_candidate(const TaigiReply *reply, size_t index, size_t row);
size_t taigi_reply_table_label_count(const TaigiReply *reply, size_t index);
const char *taigi_reply_table_label(const TaigiReply *reply, size_t index, size_t position);
uint32_t taigi_reply_table_cursor(const TaigiReply *reply, size_t index);
/* false = no highlight (the Telex guide). */
bool taigi_reply_table_cursor_visible(const TaigiReply *reply, size_t index);
uint32_t taigi_reply_table_page_size(const TaigiReply *reply, size_t index);
bool taigi_reply_table_vertical(const TaigiReply *reply, size_t index);
void taigi_reply_free(TaigiReply *reply);

#ifdef __cplusplus
}
#endif

#endif /* TAIGIKEYBOARD_H */
