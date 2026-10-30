#[cfg(target_os = "linux")]
pub fn set_linux_always_on_top(pinned: bool) {
    std::thread::spawn(move || {
        // 1. Try KDE Plasma KWin DBus scripting (Native Wayland & X11)
        let qdbus_cmd = if std::path::Path::new("/usr/bin/qdbus6").exists() {
            Some("/usr/bin/qdbus6")
        } else if std::path::Path::new("/usr/bin/qdbus").exists() {
            Some("/usr/bin/qdbus")
        } else {
            None
        };

        if let Some(qdbus) = qdbus_cmd {
            let script = format!(
                r#"let list = workspace.windowList();
for (let i = 0; i < list.length; i++) {{
    let w = list[i];
    let is_aeon = (w.caption && w.caption.indexOf("AEON-CUT") !== -1) ||
                  (w.resourceClass && w.resourceClass.indexOf("aeon") !== -1);
    if (is_aeon) {{
        w.keepAbove = {pinned};
    }}
}}"#,
                pinned = pinned
            );

            let temp_path =
                std::env::temp_dir().join(format!("aeon_kwin_pin_{}.js", std::process::id()));
            if std::fs::write(&temp_path, script).is_ok()
                && let Ok(out) = std::process::Command::new(qdbus)
                    .args([
                        "org.kde.KWin",
                        "/Scripting",
                        "loadScript",
                        &temp_path.to_string_lossy(),
                    ])
                    .output()
                && out.status.success()
            {
                let script_id = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let script_obj = format!("/Scripting/Script{}", script_id);
                let _ = std::process::Command::new(qdbus)
                    .args(["org.kde.KWin", &script_obj, "run"])
                    .output();
                let _ = std::process::Command::new(qdbus)
                    .args(["org.kde.KWin", &script_obj, "stop"])
                    .output();
            }
            let _ = std::fs::remove_file(&temp_path);
        }

        // 2. Fallback for X11 / XWayland via wmctrl
        let state_action = if pinned { "add,above" } else { "remove,above" };
        let _ = std::process::Command::new("wmctrl")
            .args(["-r", "AEON-CUT", "-b", state_action])
            .output();
    });
}

#[cfg(not(target_os = "linux"))]
pub fn set_linux_always_on_top(_pinned: bool) {}
