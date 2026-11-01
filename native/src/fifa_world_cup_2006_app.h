#pragma once

#include "fifa_world_cup_2006_init.h"

#include <rex/cvar.h>
#include <rex/filesystem.h>
#include <rex/rex_app.h>
#include <rex/system/kernel_state.h>
#include <rex/system/util/xdbf_utils.h>
#include <rex/ui/window.h>

class FifaWorldCup2006App : public rex::ReXApp {
 public:
  using rex::ReXApp::ReXApp;

  static std::unique_ptr<rex::ui::WindowedApp> Create(
      rex::ui::WindowedAppContext& ctx) {
    return std::unique_ptr<FifaWorldCup2006App>(
        new FifaWorldCup2006App(ctx, "fifa_world_cup_2006", PPCImageConfig));
  }

  void OnPreSetup(rex::RuntimeConfig& config) override {
    config.gpu_plugin = "xenos";
    if (rex::cvar::GetFlagSource("mnk_mode") == rex::cvar::Source::kDefault) {
      rex::cvar::SetFlagByName("mnk_mode", "true");
    }
  }

  void OnPostSetup() override {
    if (rex::cvar::GetFlagSource("readback_resolve") == rex::cvar::Source::kDefault) {
      rex::cvar::SetFlagByName("readback_resolve", "full");
    }
  }

  void OnPostLoadXexImage() override {
    const auto metadata = runtime()->kernel_state()->title_xdbf();
    const auto title = metadata.is_valid() ? metadata.title() : std::string{};
    window()->SetTitle(title.empty() ? "2006 FIFA World Cup" : title);
    if (metadata.is_valid()) {
      const auto icon = metadata.icon();
      if (icon) window()->SetIcon(icon.buffer, icon.size);
    }
  }

  void OnConfigurePaths(rex::PathConfig& paths) override {
    if (paths.game_data_root.empty()) {
      paths.game_data_root = rex::filesystem::GetExecutableFolder() / "prepared" / "game";
    }
  }
};
