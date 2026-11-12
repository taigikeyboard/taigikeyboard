/*
 * See engine.h. Every framework call here has its origin in fcitx5-rime's
 * rimestate.cpp / rimeengine.cpp (the reference addon); the decisions are
 * the Rust core's.
 */
#include "engine.h"

#include <fcitx-utils/i18n.h>
#include <fcitx-utils/key.h>
#include <fcitx-utils/keysym.h>
#include <fcitx-utils/log.h>
#include <fcitx-utils/utf8.h>
#include <fcitx/event.h>
#include <fcitx/inputpanel.h>
#include <fcitx/statusarea.h>
#include <fcitx/text.h>
#include <fcitx/userinterface.h>
#include <fcitx/userinterfacemanager.h>
#include <algorithm>
#include <memory>
#include <string>
#include <vector>

namespace fcitx::taigi {

FCITX_DEFINE_LOG_CATEGORY(taigi_log, "taigikeyboard");
/* No log leaves a release build (`security-rules.md`; the Rust side installs
 * no logger either, `taigi_linux_platform::install_debug_logger`). CMake's
 * Release type defines NDEBUG; `while (false)` keeps the streamed operands
 * type-checked and compiles them away. */
#ifdef NDEBUG
#define TAIGI_DEBUG() while (false) FCITX_LOGC(taigi_log, Debug)
#define TAIGI_ERROR() while (false) FCITX_LOGC(taigi_log, Error)
#else
#define TAIGI_DEBUG() FCITX_LOGC(taigi_log, Debug)
#define TAIGI_ERROR() FCITX_LOGC(taigi_log, Error)
#endif

namespace {

/* The Fcitx5 KeyState bits are the X11 / IBus ones the Rust core reads
 * (Shift 1<<0, Ctrl 1<<2, Alt/Mod1 1<<3, Mod4 1<<6, Super 1<<26); a release
 * is a separate flag on the event, added here as the IBus release bit
 * (fcitx5-rime does the same: `intStates |= (1 << 30)`). */
uint32_t statesFor(const KeyEvent &event) {
    uint32_t states = static_cast<uint32_t>(event.rawKey().states());
    if (event.isRelease()) {
        states |= TAIGI_STATE_RELEASE;
    }
    return states;
}

/* A string the library allocated, copied and freed. */
std::string takeString(char *text) {
    if (!text) {
        return {};
    }
    std::string copy(text);
    taigi_string_free(text);
    return copy;
}

/* The sub-config the configure row names. */
constexpr char kSettingsSubConfig[] = "settings";

} // namespace

// ---------------------------------------------------------------------------
// State — one input context
// ---------------------------------------------------------------------------

State::State(Engine *engine, InputContext &ic) : engine_(engine), ic_(ic) {
    handle_ = taigi_engine_new(engine_->runtime());
    if (!handle_) {
        TAIGI_ERROR() << "taigi_engine_new answered null";
    }
}

State::~State() { taigi_engine_free(handle_); }

/* The field's flags, read per key rather than on activate: a browser moves
 * focus between fields inside one input context, flipping only the flags —
 * where IBus re-sends SetCapabilities / ContentType. The surrounding text
 * too, which fcitx5-gtk re-sends before every key press: the auto-space swap
 * checks it before it deletes (#866). */
void State::syncFieldFlags() {
    const auto flags = ic_.capabilityFlags();
    taigi_engine_set_capabilities(
        handle_, flags.test(CapabilityFlag::SurroundingText) ? TAIGI_CAP_SURROUNDING_TEXT : 0);
    taigi_engine_set_password_field(handle_, flags.test(CapabilityFlag::Password));
    const auto &surrounding = ic_.surroundingText();
    taigi_engine_set_surrounding_text(handle_,
                                      surrounding.isValid() ? surrounding.text().c_str() : nullptr,
                                      surrounding.cursor(), surrounding.anchor());
}

void State::keyEvent(KeyEvent &event) {
    if (!handle_) {
        return;
    }
    syncFieldFlags();
    TaigiReply *reply = taigi_engine_key(handle_, event.rawKey().sym(), event.rawKey().code(),
                                         statesFor(event));
    if (!reply) {
        return;
    }
    const bool handled = taigi_reply_handled(reply);
    replay(reply);
    if (handled) {
        event.filterAndAccept();
    }
}

void State::beginSession() {
    if (!handle_) {
        return;
    }
    replay(taigi_engine_begin_session(handle_));
}

void State::endSession() {
    if (!handle_) {
        return;
    }
    replay(taigi_engine_end_session(handle_));
}

void State::navigate(uint32_t direction) {
    if (!handle_) {
        return;
    }
    replay(taigi_engine_navigate(handle_, direction));
}

void State::click(uint32_t position) {
    if (!handle_) {
        return;
    }
    replay(taigi_engine_click(handle_, position));
}

void State::menuActivate(const std::string &id) {
    if (!handle_) {
        return;
    }
    replay(taigi_engine_menu_activate(handle_, id.c_str()));
}

/* The reply, entry by entry, onto the input context; one UI update at the
 * end (fcitx5-rime `updateUI`). */
void State::replay(TaigiReply *reply) {
    if (!reply) {
        return;
    }
    auto &panel = ic_.inputPanel();
    bool preeditChanged = false;
    bool panelChanged = false;
    const size_t count = taigi_reply_count(reply);
    for (size_t i = 0; i < count; ++i) {
        switch (taigi_reply_kind(reply, i)) {
        case TAIGI_EMIT_PREEDIT: {
            Text preedit(taigi_reply_text(reply, i), TextFormatFlag::Underline);
            preedit.setCursor(static_cast<int>(
                utf8::ncharByteLength(preedit.toString().begin(), taigi_reply_caret(reply, i))));
            if (ic_.capabilityFlags().test(CapabilityFlag::Preedit)) {
                panel.setClientPreedit(preedit);
                panel.setPreedit(Text());
            } else {
                /* A client that cannot draw a preedit (a terminal): the panel
                 * shows it above the candidates instead. */
                panel.setClientPreedit(Text());
                panel.setPreedit(preedit);
            }
            preeditChanged = true;
            panelChanged = true;
            break;
        }
        case TAIGI_EMIT_CLEAR_PREEDIT:
            panel.setClientPreedit(Text());
            panel.setPreedit(Text());
            preeditChanged = true;
            panelChanged = true;
            break;
        case TAIGI_EMIT_COMMIT:
            ic_.commitString(taigi_reply_text(reply, i));
            break;
        case TAIGI_EMIT_DELETE_SURROUNDING:
            ic_.deleteSurroundingText(taigi_reply_delete_offset(reply, i),
                                      taigi_reply_delete_count(reply, i));
            break;
        case TAIGI_EMIT_LOOKUP_TABLE:
            showCandidates(reply, i);
            panelChanged = true;
            break;
        case TAIGI_EMIT_HIDE_LOOKUP_TABLE:
            panel.setCandidateList(nullptr);
            taigi_popup_hide();
            panelChanged = true;
            break;
        case TAIGI_EMIT_MODE_CHANGED:
            /* The label and the icon are re-read through subModeLabelImpl /
             * subModeIconImpl; the status area redraws them. */
            ic_.updateUserInterface(UserInterfaceComponent::StatusArea);
            break;
        case TAIGI_EMIT_ANNOUNCE_MODE:
            /* The mode flash of the other desktops: the framework's own
             * "input method + sub-mode" notice, timed by it. */
            engine_->instance()->showInputMethodInformation(&ic_);
            break;
        default:
            TAIGI_ERROR() << "unknown reply kind " << taigi_reply_kind(reply, i);
            break;
        }
    }
    taigi_reply_free(reply);
    if (preeditChanged) {
        ic_.updatePreedit();
    }
    if (panelChanged) {
        ic_.updateUserInterface(UserInterfaceComponent::InputPanel);
    }
}

/* The candidate window, not the framework's list: classic UI stays the
 * current UI (preedit, tray, every other input method) and would otherwise
 * draw a second list from the same reply. Clicks come back through
 * `Engine::watchPopup`. */
void State::showCandidates(const TaigiReply *reply, size_t index) {
    ic_.inputPanel().setCandidateList(nullptr);
    const auto &rect = ic_.cursorRect();
    taigi_popup_show(reply, index, rect.left(), rect.top(), rect.width(), rect.height());
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

SettingsConfig::SettingsConfig(std::string title)
    : settings_(this, "Settings", std::move(title),
                std::string("fcitx://config/addon/taigikeyboard/") + kSettingsSubConfig) {}

Engine::Engine(Instance *instance)
    : instance_(instance),
      factory_([this](InputContext &ic) { return new State(this, ic); }) {
    runtime_ = taigi_runtime_new();
    if (!runtime_) {
        TAIGI_ERROR() << "taigi_runtime_new answered null; the engine will type nothing";
    }
    TAIGI_DEBUG() << "taigikeyboard " << taigi_version() << " loaded";
    instance_->inputContextManager().registerProperty("taigikeyboardState", &factory_);
    buildMenu();
    watchPopup();
}

/* The window's fd, for the life of the engine. A click selects; a page
 * turn is the same page up / page down the old list sent. The callback
 * returns true so the watch stays. */
void Engine::watchPopup() {
    const int fd = taigi_popup_fd();
    if (fd < 0) {
        TAIGI_ERROR() << "candidate window notify pipe is unavailable";
        return;
    }
    popup_watch_ = instance_->eventLoop().addIOEvent(
        fd, IOEventFlags{IOEventFlag::In}, [this](EventSourceIO *, int, IOEventFlags) {
            uint32_t kind = 0;
            uint32_t position = 0;
            while (taigi_popup_poll(&kind, &position)) {
                auto *ic = instance_->mostRecentInputContext();
                if (!ic) {
                    continue;
                }
                auto *current = state(ic);
                if (!current) {
                    continue;
                }
                if (kind == TAIGI_POPUP_CLICK) {
                    current->click(position);
                } else if (kind == TAIGI_POPUP_PAGE) {
                    current->navigate(position == 0 ? TAIGI_NAVIGATE_PAGE_UP : TAIGI_NAVIGATE_PAGE_DOWN);
                }
            }
            return true;
        });
}

Engine::~Engine() {
    /* Drop the watch before the states it calls into. */
    popup_watch_.reset();
    /* Every state (and its engine handle) goes with the property registration;
     * the runtime is freed last, as the header requires. */
    for (auto &row : menu_) {
        instance_->userInterfaceManager().unregisterAction(&row->action);
    }
    menu_.clear();
    factory_.unregister();
    taigi_runtime_free(runtime_);
}

/* One action per row, registered under a stable name; the titles are
 * filled by refreshMenu. Separators are rows too (`setSeparator`). The
 * configure row takes the Settings row's title. */
void Engine::buildMenu() {
    TaigiMenu *menu = taigi_runtime_menu(runtime_);
    if (!menu) {
        return;
    }
    const size_t count = taigi_menu_count(menu);
    for (size_t i = 0; i < count; ++i) {
        auto row = std::make_unique<MenuRow>();
        if (taigi_menu_is_separator(menu, i)) {
            row->action.setSeparator(true);
        } else {
            row->id = taigi_menu_id(menu, i);
            if (row->id == kSettingsSubConfig) {
                config_ = std::make_unique<SettingsConfig>(taigi_menu_title(menu, i));
            }
            const std::string id = row->id;
            row->action.connect<SimpleAction::Activated>([this, id](InputContext *ic) {
                if (auto *s = state(ic)) {
                    s->menuActivate(id);
                }
            });
        }
        instance_->userInterfaceManager().registerAction("taigikeyboard-menu-" + std::to_string(i),
                                                         &row->action);
        menu_.push_back(std::move(row));
    }
    taigi_menu_free(menu);
}

/* The rows re-titled from the core (display language, recorded chords) and
 * placed in the context's status area. */
void Engine::refreshMenu(InputContext &ic) {
    TaigiMenu *menu = taigi_runtime_menu(runtime_);
    if (menu) {
        const size_t count = std::min(taigi_menu_count(menu), menu_.size());
        for (size_t i = 0; i < count; ++i) {
            if (menu_[i]->action.isSeparator()) {
                continue;
            }
            menu_[i]->action.setShortText(taigi_menu_title(menu, i));
            menu_[i]->action.setLongText(taigi_menu_detail(menu, i));
        }
        taigi_menu_free(menu);
    }
    auto &statusArea = ic.statusArea();
    statusArea.clearGroup(StatusGroup::InputMethod);
    for (auto &row : menu_) {
        statusArea.addAction(StatusGroup::InputMethod, &row->action);
    }
}

void Engine::activate(const InputMethodEntry & /*entry*/, InputContextEvent &event) {
    auto *ic = event.inputContext();
    refreshMenu(*ic);
    if (auto *s = state(ic)) {
        s->beginSession();
    }
}

void Engine::deactivate(const InputMethodEntry &entry, InputContextEvent &event) {
    /* A switch to another input method writes what is on screen, as the IBus
     * daemon does when it unsets an engine (bus/inputcontext.c
     * `bus_input_context_unset_engine` → `clear_preedit_text(TRUE)` under
     * PREEDIT_COMMIT). Focus loss needs nothing here: Fcitx5 has written the
     * client preedit already (5.1.7 instance.cpp:1037), as fcitx5-gtk does
     * itself before a Reset (`fcitx_im_context_reset`). */
    if (event.type() == EventType::InputContextSwitchInputMethod) {
        auto *ic = event.inputContext();
        const auto commit = ic->inputPanel().clientPreedit().toStringForCommit();
        if (!commit.empty()) {
            ic->commitString(commit);
        }
    }
    reset(entry, event);
}

void Engine::keyEvent(const InputMethodEntry & /*entry*/, KeyEvent &keyEvent) {
    if (auto *s = state(keyEvent.inputContext())) {
        s->keyEvent(keyEvent);
    }
}

void Engine::reset(const InputMethodEntry & /*entry*/, InputContextEvent &event) {
    auto *ic = event.inputContext();
    if (auto *s = state(ic)) {
        s->endSession();
    }
    ic->inputPanel().reset();
    ic->updatePreedit();
    ic->updateUserInterface(UserInterfaceComponent::InputPanel);
    /* A client's reset leaves the session running, unlike the focus loss and
     * the switch that `deactivate` sends here. */
    if (event.type() == EventType::InputContextReset) {
        if (auto *s = state(ic)) {
            s->beginSession();
        }
    }
}

/* The tray / kimpanel text (`Instance::inputMethodLabel`) and the compact
 * notice: the short symbol the IBus shell shows (roadmap L6). */
std::string Engine::subModeLabelImpl(const InputMethodEntry & /*entry*/, InputContext & /*ic*/) {
    return takeString(taigi_runtime_mode_symbol(runtime_));
}

/* The same state as an icon theme name, installed under hicolor. */
std::string Engine::subModeIconImpl(const InputMethodEntry & /*entry*/, InputContext & /*ic*/) {
    return takeString(taigi_runtime_mode_icon(runtime_));
}

/* The full notice: the romanization and the display mode, the text the
 * mode flash of the other desktops announces. */
std::string Engine::subMode(const InputMethodEntry & /*entry*/, InputContext & /*ic*/) {
    return takeString(taigi_runtime_mode_label(runtime_));
}

void Engine::setSubConfig(const std::string &path, const RawConfig & /*config*/) {
    if (path == kSettingsSubConfig && !taigi_open_settings()) {
        TAIGI_ERROR() << "the settings window could not be started";
    }
}

AddonInstance *EngineFactory::create(AddonManager *manager) {
    return new Engine(manager->instance());
}

} // namespace fcitx::taigi

FCITX_ADDON_FACTORY(fcitx::taigi::EngineFactory)
