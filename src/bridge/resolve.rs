use std::env;
use std::path::PathBuf;
use std::process::Command;
use crate::models::{SelectedClipInfo, SpeechSegment, TimelineStatus};

pub struct ResolveBridge {
    script_path: PathBuf,
    python_cmd: String,
}

impl ResolveBridge {
    pub fn new() -> Self {
        // 1. Locate the bridge script relative to current dir or executable directory
        let mut script_path = PathBuf::from("bridge/resolve_bridge.py");
        if !script_path.exists() {
            if let Ok(mut exe_dir) = env::current_exe() {
                exe_dir.pop(); // Remove exe name
                let candidate1 = exe_dir.join("bridge").join("resolve_bridge.py");
                let candidate2 = exe_dir.join("resolve_bridge.py");
                if candidate1.exists() {
                    script_path = candidate1;
                } else if candidate2.exists() {
                    script_path = candidate2;
                }
            }
        }

        // 2. Detect available Python command across Linux, macOS, and Windows
        let python_cmd = Self::detect_python_cmd();

        Self {
            script_path,
            python_cmd,
        }
    }

    fn detect_python_cmd() -> String {
        // Try python3 first (standard on Linux/macOS)
        if Command::new("python3").arg("--version").output().is_ok() {
            return "python3".to_string();
        }
        // Try python (standard on Windows)
        if Command::new("python").arg("--version").output().is_ok() {
            return "python".to_string();
        }
        // Try Windows Python Launcher (py)
        if Command::new("py").arg("-3").arg("--version").output().is_ok() {
            return "py".to_string();
        }
        // Fallback default
        "python3".to_string()
    }

    fn build_command(&self) -> Command {
        let mut cmd = Command::new(&self.python_cmd);
        if self.python_cmd == "py" {
            cmd.arg("-3");
        }
        cmd.arg(&self.script_path);
        cmd
    }

    pub fn get_status(&self) -> Result<TimelineStatus, String> {
        let mut cmd = self.build_command();
        cmd.arg("--action").arg("status");

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to spawn Python bridge ({}): {}", self.python_cmd, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Bridge process error: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str::<TimelineStatus>(&stdout)
            .map_err(|e| format!("JSON parse error ({}): {}", e, stdout))
    }

    pub fn get_selected_clip(&self) -> Result<SelectedClipInfo, String> {
        let mut cmd = self.build_command();
        cmd.arg("--action").arg("get_selected_clip");

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to spawn Python bridge ({}): {}", self.python_cmd, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Bridge process error: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str::<SelectedClipInfo>(&stdout)
            .map_err(|e| format!("JSON parse error ({}): {}", e, stdout))
    }

    pub fn apply_clean_timeline(
        &self,
        segments: &[SpeechSegment],
        suffix: &str,
    ) -> Result<String, String> {
        let json_data = serde_json::to_string(segments)
            .map_err(|e| format!("Failed to serialize segments: {}", e))?;

        let mut cmd = self.build_command();
        cmd.arg("--action")
            .arg("create_clean_timeline")
            .arg("--segments-json")
            .arg(json_data)
            .arg("--suffix")
            .arg(suffix);

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to execute Python bridge: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let resp: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| format!("Invalid JSON response ({}): {}", e, stdout))?;

        if let Some(err) = resp.get("error").and_then(|e| e.as_str()) {
            return Err(err.to_string());
        }

        Ok(resp
            .get("new_timeline_name")
            .and_then(|v| v.as_str())
            .unwrap_or("New Timeline")
            .to_string())
    }

    pub fn apply_active_timeline(&self, segments: &[SpeechSegment]) -> Result<usize, String> {
        let json_data = serde_json::to_string(segments)
            .map_err(|e| format!("Failed to serialize segments: {}", e))?;

        let mut cmd = self.build_command();
        cmd.arg("--action")
            .arg("append_to_active")
            .arg("--segments-json")
            .arg(json_data);

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to execute Python bridge: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let resp: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| format!("Invalid JSON response ({}): {}", e, stdout))?;

        if let Some(err) = resp.get("error").and_then(|e| e.as_str()) {
            return Err(err.to_string());
        }

        Ok(resp.get("segments_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize)
    }

    pub fn apply_markers(&self, segments: &[SpeechSegment]) -> Result<usize, String> {
        let json_data = serde_json::to_string(segments)
            .map_err(|e| format!("Failed to serialize segments: {}", e))?;

        let mut cmd = self.build_command();
        cmd.arg("--action")
            .arg("add_markers")
            .arg("--segments-json")
            .arg(json_data);

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to execute Python bridge: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let resp: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| format!("Invalid JSON response ({}): {}", e, stdout))?;

        if let Some(err) = resp.get("error").and_then(|e| e.as_str()) {
            return Err(err.to_string());
        }

        Ok(resp.get("markers_added").and_then(|v| v.as_u64()).unwrap_or(0) as usize)
    }
}
