use std::path::Path;
use std::fs;

const AUTOUPDATE_SERVICE: &str = "com.anvil.autoupdate";

pub fn execute(config_dir: &Path, args: Vec<String>) -> i32 {
    let subcommand = args.first().map(|s| s.as_str()).unwrap_or("status");

    match subcommand {
        "enable" => enable_autoupdate(config_dir, &args[1..]),
        "disable" => disable_autoupdate(),
        "status" => show_status(),
        "set" => set_schedule(&args[1..]),
        _ => {
            println!("Usage: anvil autoupdate <subcommand>");
            println!("Subcommands:");
            println!("  anvil autoupdate status    Show current autoupdate status");
            println!("  anvil autoupdate enable    Enable automatic updates");
            println!("  anvil autoupdate disable   Disable automatic updates");
            println!("  anvil autoupdate set <time> Set update interval (e.g., 'daily', 'weekly', '12h', '6h')");
            1
        }
    }
}

fn detect_scheduler() -> &'static str {
    if std::process::Command::new("which")
        .arg("systemctl")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return "systemd";
    }

    if std::process::Command::new("which")
        .arg("launchctl")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return "launchd";
    }

    if std::process::Command::new("which")
        .arg("crontab")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return "cron";
    }

    "unknown"
}

fn show_status() -> i32 {
    let scheduler = detect_scheduler();
    println!("Detected scheduler: {}", scheduler);

    match scheduler {
        "systemd" => {
            let result = std::process::Command::new("systemctl")
                .args(["is-active", "--user", "anvil-updater.timer"])
                .output();

            if let Ok(output) = result {
                if output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "active" {
                    println!("Status: Active");
                    let timer_result = std::process::Command::new("systemctl")
                        .args(["list-timers", "--user", "--all", "-o", "brief"])
                        .output();
                    if let Ok(timer_output) = timer_result {
                        for line in String::from_utf8_lossy(&timer_output.stdout).lines() {
                            if line.contains("anvil-updater") {
                                println!("Timer: {}", line.trim());
                            }
                        }
                    }
                } else {
                    println!("Status: Inactive");
                }
            } else {
                println!("Status: Inactive");
            }
        }
        "launchd" => {
            let plist_path = dirs::home_dir()
                .map(|h| h.join("Library/LaunchAgents/com.anvil.autoupdate.plist"))
                .unwrap_or_default();

            if plist_path.exists() {
                println!("Status: Active (plist exists at {:?})", plist_path);
            } else {
                println!("Status: Inactive");
            }
        }
        "cron" => {
            let result = std::process::Command::new("crontab")
                .args(["-l"])
                .output();

            if let Ok(output) = result {
                let crontab = String::from_utf8_lossy(&output.stdout);
                if crontab.contains("anvil") {
                    println!("Status: Active");
                    for line in crontab.lines() {
                        if line.contains("anvil") {
                            println!("Entry: {}", line);
                        }
                    }
                } else {
                    println!("Status: Inactive");
                }
            } else {
                println!("Status: Inactive");
            }
        }
        _ => {
            println!("Status: Unknown scheduler, cannot configure");
        }
    }

    0
}

fn enable_autoupdate(_config_dir: &Path, args: &[String]) -> i32 {
    let scheduler = detect_scheduler();

    if scheduler == "unknown" {
        println!("Error: Could not detect a supported scheduler.");
        println!("Supported schedulers: systemd, launchd, cron");
        return 1;
    }

    let interval = if let Some(interval_arg) = args.first() {
        interval_arg.clone()
    } else {
        "daily".to_string()
    };

    println!("Enabling autoupdate with {} scheduler...", scheduler);
    println!("Update interval: {}", interval);

    let schedule_expr = match interval.as_str() {
        "hourly" => "@hourly".to_string(),
        "daily" => "@daily".to_string(),
        "weekly" => "@weekly".to_string(),
        "12h" => "*/12 * * * *".to_string(),
        "6h" => "*/6 * * * *".to_string(),
        "3h" => "*/3 * * * *".to_string(),
        "1h" | "60min" => "0 * * * *".to_string(),
        _ => {
            if interval.ends_with('h') || interval.ends_with("min") {
                interval.clone()
            } else {
                println!("Warning: Unknown interval '{}', defaulting to daily", interval);
                "@daily".to_string()
            }
        }
    };

    match scheduler {
        "systemd" => enable_systemd_timer(&schedule_expr),
        "launchd" => enable_launchd_plist(&schedule_expr),
        "cron" => enable_cron(&schedule_expr),
        _ => {
            println!("Error: Unsupported scheduler");
            1
        }
    }
}

fn enable_systemd_timer(schedule: &str) -> i32 {
    let service_content = format!(r#"[Unit]
Description=Anvil Auto Update

[Service]
Type=oneshot
ExecStart=/bin/bash -c 'anvil upgrade'
"#);

    let timer_content = if schedule.starts_with('@') {
        let timer_spec = match schedule {
            "@hourly" => "hourly",
            "@daily" => "daily",
            "@weekly" => "weekly",
            "@monthly" => "monthly",
            _ => "daily",
        };
        format!(r#"[Unit]
Description=Anvil Auto Update Timer

[Timer]
OnCalendar={}
Persistent=true

[Install]
WantedBy=timers.target
"#, timer_spec)
    } else {
        format!(r#"[Unit]
Description=Anvil Auto Update Timer

[Timer]
OnCalendar={}
Persistent=true

[Install]
WantedBy=timers.target
"#, schedule)
    };

    let home = dirs::home_dir().unwrap_or_default();
    let config_dir = home.join(".config/systemd/user");

    if let Err(e) = fs::create_dir_all(&config_dir) {
        println!("Error creating config directory: {}", e);
        return 1;
    }

    let service_path = config_dir.join("anvil-updater.service");
    let timer_path = config_dir.join("anvil-updater.timer");

    if let Err(e) = fs::write(&service_path, service_content) {
        println!("Error writing service file: {}", e);
        return 1;
    }

    if let Err(e) = fs::write(&timer_path, timer_content) {
        println!("Error writing timer file: {}", e);
        return 1;
    }

    let _ = std::process::Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output();

    let _ = std::process::Command::new("systemctl")
        .args(["--user", "enable", "anvil-updater.timer"])
        .output();

    let result = std::process::Command::new("systemctl")
        .args(["--user", "start", "anvil-updater.timer"])
        .output();

    if result.map(|o| o.status.success()).unwrap_or(false) {
        println!("Enabled! Anvil will update automatically.");
        0
    } else {
        println!("Warning: Timer may not have started properly. Check with 'systemctl --user status anvil-updater.timer'");
        1
    }
}

fn enable_launchd_plist(schedule: &str) -> i32 {
    let home = dirs::home_dir().unwrap_or_default();
    let launch_agents = home.join("Library/LaunchAgents");
    let plist_path = launch_agents.join("com.anvil.autoupdate.plist");

    if let Err(e) = fs::create_dir_all(&launch_agents) {
        println!("Error creating LaunchAgents directory: {}", e);
        return 1;
    }

    let plist_content = if schedule == "@daily" || schedule == "daily" {
        format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.anvil.autoupdate</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/bash</string>
        <string>-c</string>
        <string>anvil upgrade</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict>
        <key>Hour</key>
        <integer>3</integer>
        <key>Minute</key>
        <integer>0</integer>
    </dict>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#)
    } else {
        let minutes = if schedule.contains("6h") {
            360
        } else if schedule.contains("3h") {
            180
        } else if schedule.contains("1h") {
            60
        } else {
            1440
        };

        format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.anvil.autoupdate</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/bash</string>
        <string>-c</string>
        <string>anvil upgrade</string>
    </array>
    <key>StartInterval</key>
    <integer>{}</integer>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#, minutes)
    };

    if let Err(e) = fs::write(&plist_path, plist_content) {
        println!("Error writing plist file: {}", e);
        return 1;
    }

    let result = std::process::Command::new("launchctl")
        .args(["load", plist_path.to_str().unwrap_or_default()])
        .output();

    if result.map(|o| o.status.success()).unwrap_or(false) {
        println!("Enabled! Anvil will update automatically.");
        0
    } else {
        println!("Enabled! Log out/in or run 'launchctl load {}' to start.", plist_path.display());
        0
    }
}

fn enable_cron(schedule: &str) -> i32 {
    let cron_entry = if schedule == "@daily" || schedule == "daily" {
        "0 3 * * *".to_string()
    } else if schedule == "@weekly" || schedule == "weekly" {
        "0 3 * * 0".to_string()
    } else if schedule == "@hourly" || schedule == "hourly" {
        "0 * * * *".to_string()
    } else {
        schedule.to_string()
    };

    let update_cmd = "anvil upgrade";
    let new_line = format!("{} {}", cron_entry, update_cmd);

    let current_crontab = std::process::Command::new("crontab")
        .args(["-l"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let mut lines: Vec<&str> = current_crontab.lines().collect();
    lines.retain(|l| !l.contains("anvil"));

    let new_crontab = if lines.is_empty() {
        new_line
    } else {
        let mut content = lines.join("\n");
        content.push_str("\n");
        content.push_str(&new_line);
        content
    };

    let result = std::process::Command::new("bash")
        .args(["-c", &format!("echo '{}' | crontab -", new_crontab)])
        .output();

    if result.map(|o| o.status.success()).unwrap_or(false) {
        println!("Enabled! Anvil will update automatically with schedule: {}", cron_entry);
        0
    } else {
        println!("Error: Failed to install crontab entry");
        1
    }
}

fn disable_autoupdate() -> i32 {
    let scheduler = detect_scheduler();

    match scheduler {
        "systemd" => {
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "stop", "anvil-updater.timer"])
                .output();
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "disable", "anvil-updater.timer"])
                .output();

            let home = dirs::home_dir().unwrap_or_default();
            let service_path = home.join(".config/systemd/user/anvil-updater.service");
            let timer_path = home.join(".config/systemd/user/anvil-updater.timer");

            let _ = fs::remove_file(service_path);
            let _ = fs::remove_file(timer_path);

            println!("Disabled systemd timer.");
        }
        "launchd" => {
            let plist_path = dirs::home_dir()
                .map(|h| h.join("Library/LaunchAgents/com.anvil.autoupdate.plist"))
                .unwrap_or_default();

            let _ = std::process::Command::new("launchctl")
                .args(["unload", plist_path.to_str().unwrap_or_default()])
                .output();
            let _ = fs::remove_file(&plist_path);

            println!("Disabled launchd job.");
        }
        "cron" => {
            let current_crontab = std::process::Command::new("crontab")
                .args(["-l"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_default();

            let lines: Vec<&str> = current_crontab.lines()
                .filter(|l| !l.contains("anvil"))
                .collect();

            if lines.is_empty() {
                let _ = std::process::Command::new("bash")
                    .args(["-c", "crontab -r"])
                    .output();
            } else {
                let new_crontab = lines.join("\n");
                let _ = std::process::Command::new("bash")
                    .args(["-c", &format!("echo '{}' | crontab -", new_crontab)])
                    .output();
            }

            println!("Disabled cron job.");
        }
        _ => {
            println!("Unknown scheduler, nothing to disable.");
        }
    }

    0
}

fn set_schedule(args: &[String]) -> i32 {
    if args.is_empty() {
        println!("Usage: anvil autoupdate set <interval>");
        println!("Intervals: hourly, daily, weekly, 12h, 6h, 3h, 1h");
        return 1;
    }

    let interval = &args[0];
    enable_autoupdate(Path::new(""), &[interval.clone()])
}