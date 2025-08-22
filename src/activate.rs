use std::process::Command;

pub fn activate_fokus_tty() -> Result<(), Box<dyn std::error::Error>> {
    let ps_output = Command::new("ps")
        .arg("ax")
        .arg("-o")
        .arg("pid,tty,comm")
        .output()?;

    let ps_string = String::from_utf8(ps_output.stdout)?;

    // Look for fokus process (prefer target/debug/fokus over fokus)
    let mut fokus_tty = None;

    for line in ps_string.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let tty = parts[1];
            let command = parts[2];

            if command.contains("fokus") {
                if command.contains("target/debug/fokus") {
                    // Prefer debug version
                    fokus_tty = Some(format!("/dev/{}", tty));
                    break; 
                } else if command.ends_with("fokus") {
                    fokus_tty = Some(format!("/dev/{}", tty));
                }
            }
        }
    }

    if let Some(tty_target) = fokus_tty {
        #[cfg(target_os = "macos")]
        {
            let script = format!(
                "tell application \"iTerm2\"
                repeat with theWindow in windows
                    repeat with theTab in tabs of theWindow
                        repeat with theSession in sessions of theTab
                            set currentTTY to (tty of theSession as string)
                            if currentTTY is equal to \"{}\" then
                                set index of theWindow to 1
                                tell theWindow
                                    select theTab
                                end tell
                                tell theTab
                                    select theSession
                                end tell
                                activate
                                return
                            end if
                        end repeat
                    end repeat
                end repeat
            end tell",
                tty_target
            );

            let _ = Command::new("osascript").arg("-e").arg(script).output();
        }

        Ok(())
    } else {
        Err("No fokus process found".into())
    }
}
