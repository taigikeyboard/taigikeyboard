/*
 * The Fcitx5 shell of TaigiKeyboard: an InputMethodEngineV3 that hands
 * every key to the Rust core through the C ABI (taigikeyboard.h) and
 * replays the reply into the input context — client preedit, commit,
 * delete-surrounding, candidate list. It composes nothing itself: the same
 * `Emit` list drives the IBus shell, so a behaviour that differs between
 * the two is a shell bug (docs/architecture/linux-roadmap.md L1, 2026-09-23).
 *
 * Shape follows fcitx5-rime (src/rimeengine.h, src/rimestate.cpp): one
 * engine, one per-input-context property holding the Rust engine handle.
 */
#ifndef TAIGIKEYBOARD_FCITX5_ENGINE_H
#define TAIGIKEYBOARD_FCITX5_ENGINE_H

#include <fcitx-config/configuration.h>
#include <fcitx-config/option.h>
#include <fcitx/action.h>
#include <fcitx/addonfactory.h>
#include <fcitx/addoninstance.h>
#include <fcitx/addonmanager.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextproperty.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/instance.h>
#include <memory>
#include <string>
#include <vector>

#include "taigikeyboard.h"

namespace fcitx::taigi {

class Engine;

/* The Rust engine handle for one input context, owned by the context
 * (InputContextProperty: created on first use, destroyed with the context). */
class State final : public InputContextProperty {
public:
    State(Engine *engine, InputContext &ic);
    ~State() override;

    void keyEvent(KeyEvent &event);
    void endSession();
    void navigate(uint32_t direction);
    void click(uint32_t position);
    void menuActivate(const std::string &id);

private:
    void syncFieldFlags();
    void replay(TaigiReply *reply);
    void showCandidates(const TaigiReply *reply, size_t index);

    Engine *engine_;
    InputContext &ic_;
    ::TaigiEngine *handle_ = nullptr;
};

/* What the framework's configure button shows (fcitx5-configtool, the KDE
 * page): one row that opens the settings window, as the IBus component's
 * <setup> does. `SubConfigOption` makes the tool call `setSubConfig`
 * rather than draw a page (fcitx5-mozc's config tool row). */
class SettingsConfig final : public Configuration {
public:
    explicit SettingsConfig(std::string title);
    const char *typeName() const override { return "TaigiKeyboardConfig"; }

private:
    SubConfigOption settings_;
};

class Engine final : public InputMethodEngineV3 {
public:
    explicit Engine(Instance *instance);
    ~Engine() override;

    Instance *instance() { return instance_; }
    ::TaigiRuntime *runtime() { return runtime_; }

    void activate(const InputMethodEntry &entry, InputContextEvent &event) override;
    void deactivate(const InputMethodEntry &entry, InputContextEvent &event) override;
    void keyEvent(const InputMethodEntry &entry, KeyEvent &keyEvent) override;
    void reset(const InputMethodEntry &entry, InputContextEvent &event) override;
    /* Both the compact and the full "input method information" notices
     * read the mode (`Instance::showInputMethodInformation`, 5.1.7
     * instance.cpp:394: `subMode()` when CompactInputMethodInformation is
     * off, `subModeLabel()` otherwise). */
    std::string subMode(const InputMethodEntry &entry, InputContext &ic) override;
    std::string subModeLabelImpl(const InputMethodEntry &entry, InputContext &ic) override;
    const Configuration *getConfig() const override { return config_.get(); }
    void setSubConfig(const std::string &path, const RawConfig &config) override;

    State *state(InputContext *ic) { return ic->propertyFor(&factory_); }

private:
    /* The status-area rows (roadmap L6): one SimpleAction per menu row the
     * core lists, registered once, re-titled on every activate so a display
     * language or a chord recorded in the settings window shows on the next
     * focus (fcitx5-rime `refreshStatusArea`). */
    void buildMenu();
    void refreshMenu(InputContext &ic);

    Instance *instance_;
    ::TaigiRuntime *runtime_ = nullptr;
    FactoryFor<State> factory_;
    struct MenuRow {
        std::string id;
        SimpleAction action;
    };
    std::vector<std::unique_ptr<MenuRow>> menu_;
    std::unique_ptr<SettingsConfig> config_;
};

class EngineFactory final : public AddonFactory {
public:
    AddonInstance *create(AddonManager *manager) override;
};

} // namespace fcitx::taigi

#endif
