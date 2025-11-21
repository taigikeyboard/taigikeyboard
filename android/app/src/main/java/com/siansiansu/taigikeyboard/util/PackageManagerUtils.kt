
package com.siansiansu.taigikeyboard.util

import android.content.ComponentName
import android.content.Context
import android.content.pm.PackageManager

object PackageManagerUtils {
    private const val SETTINGS_ACTIVITY_NAME = "com.siansiansu.taigikeyboard.SettingsLauncherAlias"

    fun hideAppIcon(context: Context) {
        val pkg: PackageManager = context.packageManager
        pkg.setComponentEnabledSetting(
            ComponentName(context, SETTINGS_ACTIVITY_NAME),
            PackageManager.COMPONENT_ENABLED_STATE_DISABLED,
            PackageManager.DONT_KILL_APP
        )
    }

    fun showAppIcon(context: Context) {
        val pkg: PackageManager = context.packageManager
        pkg.setComponentEnabledSetting(
            ComponentName(context, SETTINGS_ACTIVITY_NAME),
            PackageManager.COMPONENT_ENABLED_STATE_ENABLED,
            PackageManager.DONT_KILL_APP
        )
    }
}
