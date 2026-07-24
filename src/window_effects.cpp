#include "window_effects.h"

#include <KWindowSystem/KWindowEffects>
#include <QGuiApplication>
#include <QWindow>

bool enable_aclm_blur()
{
    const auto windows = QGuiApplication::allWindows();
    const bool available = KWindowEffects::isEffectAvailable(KWindowEffects::BlurBehind);
    const bool contrastAvailable =
        KWindowEffects::isEffectAvailable(KWindowEffects::BackgroundContrast);

    for (QWindow *window : windows) {
        KWindowEffects::enableBlurBehind(window, available);
        KWindowEffects::enableBackgroundContrast(window, contrastAvailable);
    }

    return available && !windows.isEmpty();
}
