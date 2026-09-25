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
#include <fcitx/candidatelist.h>
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

/* A candidate the panel shows: selecting it (a click) only highlights, as
 * on macOS and Windows; the commit stays a key's. */
class CandidateWordImpl final : public CandidateWord {
public:
    CandidateWordImpl(State *state, uint32_t position, Text text)
        : CandidateWord(std::move(text)), state_(state), position_(position) {}

    void select(InputContext * /*ic*/) const override { state_->click(position_); }

private:
    State *state_;
    uint32_t position_;
};

/* The list with its paging and cursor moves routed through the core: the
 * panel's page arrows and scroll call `prev` / `next` / `prevCandidate` /
 * `nextCandidate` on the list itself, and a page turned only here would
 * leave the core's page — the one the slot keys and clicks index — behind.
 * Each call answers a fresh reply that rebuilds the list on the right page. */
class CandidateListImpl final : public CommonCandidateList {
public:
    explicit CandidateListImpl(State *state) : state_(state) {}

    void prev() override { state_->navigate(TAIGI_NAVIGATE_PAGE_UP); }
    void next() override { state_->navigate(TAIGI_NAVIGATE_PAGE_DOWN); }
    void prevCandidate() override { state_->navigate(TAIGI_NAVIGATE_PREVIOUS); }
    void nextCandidate() override { state_->navigate(TAIGI_NAVIGATE_NEXT); }

private:
    State *state_;
};

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
 * where IBus re-sends SetCapabilities / ContentType. */
void State::syncFieldFlags() {
    const auto flags = ic_.capabilityFlags();
    taigi_engine_set_capabilities(
        handle_, flags.test(CapabilityFlag::SurroundingText) ? TAIGI_CAP_SURROUNDING_TEXT : 0);
    taigi_engine_set_password_field(handle_, flags.test(CapabilityFlag::Password));
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
            panelChanged = true;
            break;
        case TAIGI_EMIT_MODE_CHANGED:
            /* The label is re-read through subModeLabelImpl; the status
             * area redraws it. */
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

/* The list as the panel draws it: page size = the slot keys, labels = the
 * slot keys, the highlight at the absolute index the core keeps. Paging and
 * cursor moves from the panel go back through the core (`navigate`), so
 * the list is rebuilt from the next reply rather than paged locally. */
void State::showCandidates(const TaigiReply *reply, size_t index) {
    auto list = std::make_unique<CandidateListImpl>(this);
    const size_t rows = taigi_reply_table_count(reply, index);
    const size_t labels = taigi_reply_table_label_count(reply, index);
    const uint32_t pageSize = taigi_reply_table_page_size(reply, index);
    /* A table with no labels (the Telex guide) still needs one label per
     * page position: the panel indexes the vector by row. */
    std::vector<std::string> labelTexts;
    labelTexts.reserve(labels > 0 ? labels : pageSize);
    for (size_t position = 0; position < labels; ++position) {
        labelTexts.emplace_back(std::string(taigi_reply_table_label(reply, index, position)) + " ");
    }
    for (size_t position = labels; position < pageSize; ++position) {
        labelTexts.emplace_back("");
    }
    list->setLabels(labelTexts);
    list->setPageSize(static_cast<int>(pageSize));
    list->setLayoutHint(taigi_reply_table_vertical(reply, index) ? CandidateLayoutHint::Vertical
                                                                  : CandidateLayoutHint::Horizontal);
    /* No setSelectionKey: in fcitx5 it only rewrites the labels (both calls
     * fill the same vector), so an empty KeyList would blank the slot keys
     * set above. Slot keys reach the core through keyEvent regardless. */
    for (size_t row = 0; row < rows; ++row) {
        list->append(std::make_unique<CandidateWordImpl>(
            this, static_cast<uint32_t>(row % (pageSize == 0 ? 1 : pageSize)),
            Text(taigi_reply_table_candidate(reply, index, row))));
    }
    /* The page is the highlight's: setGlobalCursorIndex alone leaves the
     * list on page 0. */
    if (taigi_reply_table_cursor_visible(reply, index)) {
        const uint32_t cursor = taigi_reply_table_cursor(reply, index);
        if (pageSize > 0 && rows > 0) {
            list->setPage(static_cast<int>(cursor / pageSize));
        }
        list->setGlobalCursorIndex(static_cast<int>(cursor));
    } else {
        list->setGlobalCursorIndex(-1);
    }
    ic_.inputPanel().setCandidateList(std::move(list));
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
}

Engine::~Engine() {
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
 * configure row takes the 設定 row's title. */
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
    refreshMenu(*event.inputContext());
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
}

/* The tray / kimpanel text (`Instance::inputMethodLabel`) and the compact
 * notice: the short symbol the IBus shell shows (roadmap L6). */
std::string Engine::subModeLabelImpl(const InputMethodEntry & /*entry*/, InputContext & /*ic*/) {
    return takeString(taigi_runtime_mode_symbol(runtime_));
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
