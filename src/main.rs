use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use core::time;
use std::{path::PathBuf, process, time::Duration};
mod bepmod;
use dioxus::desktop::{Config, WindowBuilder};

const MODS_JSON_URL: &'static str = "https://raw.githubusercontent.com/IdotNuerk/CumHeim/master/mods.json";

fn main() {
    let icon_path = std::path::PathBuf::from("icons/icon.ico");
    let icon_bytes = std::fs::read(&icon_path).expect("Failed to read icon file");
    let icon_image = image::load_from_memory(&icon_bytes)
        .expect("Failed to load icon image")
        .to_rgba8();
    let (width, height) = icon_image.dimensions();
    
    let icon = dioxus::desktop::tao::window::Icon::from_rgba(
        icon_image.into_raw(),
        width,
        height
    ).expect("Failed to create icon");
    
    let config = Config::new()
        .with_window(
            WindowBuilder::new()
                .with_title("Valheim Mod Installer")
                .with_decorations(false)
                .with_resizable(false)
                .with_inner_size(dioxus::desktop::wry::dpi::LogicalSize::new(950.0, 600.0))
                .with_window_icon(Some(icon))
                .with_decorations(true)
                
        )
        .with_menu(None);
    
    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(app);
}

#[derive(Clone, PartialEq)]
struct Mod {
    id: String,
    name: String,
    description: String,
    icon_url: String,
    download_url: String,
    version: String,
    enabled: bool,
    from: Option<String>,
    to: Option<String>,
}

#[derive(Clone, PartialEq, Deserialize, Serialize, Debug)]
struct ThunderstorePackage {
    namespace: String,
    name: String,
    full_name: String,
    owner: String,
    latest: ThunderstoreVersion, #[serde(default)]
    package_url: String,
}

#[derive(Clone, PartialEq, Deserialize, Serialize, Debug)]
struct ThunderstoreVersion {
    namespace: String,
    name: String,
    version_number: String,
    full_name: String,
    description: String,
    icon: String,
    download_url: String,
    dependencies: Vec<String>, #[serde(default)]
    downloads: i64,
    website_url: String,
}

fn app() -> Element {
    let mut status = use_signal(|| String::from("Initializing..."));
    let mut valheim_location = use_signal(|| None::<PathBuf>);
    let mut auto_detect_failed = use_signal(|| false);
    let mut mods = use_signal(|| Vec::<Mod>::new());
    let mut loading_mods = use_signal(|| false);
    let mut mods_json_info = use_signal(|| Vec::<bepmod::BepinexMod>::new() );
    let mut primary_pressed = use_signal(|| false);
    let mut secondary_pressed = use_signal(|| false);
    let mut install_is_processing = use_signal(|| false);
    let mut uninstall_is_processing = use_signal(|| false);
    
    // Find Steam on component mount
    use_effect(move || {
        if let Some(path) = find_game_directory("Valheim") {
            valheim_location.set(Some(path.clone()));
            status.set(format!("Found Valheim at: {}", path.display()));
        } else {
            auto_detect_failed.set(true);
            status.set("Valheim installation not found. Please select manually.".to_string());
        }
    });

    let select_valheim_directory = move |_| {
        spawn(async move {
            if let Some(path) = open_directory_picker() {
                // Verify this is a valid Steam directory
                if path.exists() {
                    valheim_location.set(Some(path.clone()));
                    auto_detect_failed.set(false);
                    status.set(format!("Valheim directory selected: {}", path.display()));
                } else {
                    status.set("Selected directory doesn't appear to exist".to_string());
                }
            }
        });
    };

    let uninstall_mods = move |_| {
        if uninstall_is_processing() {
            return;
        }

        spawn(async move {
            uninstall_is_processing.set(true);
            match uninstall(valheim_location()) {
                Ok(_) => { status.set("Finished uninstalling all mods.".to_string()); },
                Err(e) => {
                    status.set(format!("Error uninstalling all mods: {}", e));
                }
            }
            uninstall_is_processing.set(false);
        });
    };

    use_effect( move || {
        spawn(async move {
            loading_mods.set(true);
            status.set("Fetching mod information from Thunderstore...".to_string());
            
            let mut fetched_mods = Vec::new();

            if let Ok(mods) = get_mods_json().await {
                mods_json_info.set(mods);
            }
            
            for info in mods_json_info.iter() {
                let name = info.name.trim();
                let namespace = info.namespace.trim();
                if name.is_empty() || namespace.is_empty() {
                    continue;
                }
                
                // Fetch from Thunderstore API
                let api_url = format!("https://thunderstore.io/api/experimental/package/{}/{}/", namespace, name);
                
                match reqwest::get(&api_url).await {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.json::<ThunderstorePackage>().await {
                                Ok(package) => {
                                    fetched_mods.push(Mod {
                                        id: package.full_name.clone(),
                                        name: package.name.clone(),
                                        description: package.latest.description.clone(),
                                        icon_url: package.latest.icon.clone(),
                                        download_url: package.latest.download_url.clone(),
                                        version: package.latest.version_number.clone(),
                                        enabled: true,
                                        from: info.from.clone(),
                                        to: info.to.clone(),
                                    });
                                    status.set(format!("Loaded: {} v{}", package.name, package.latest.version_number));
                                }
                                Err(e) => {
                                    status.set(format!("Error parsing {}: {}", name, e));
                                }
                            }
                        } else {
                            status.set(format!("Could not find mod: {}", name));
                        }
                    }
                    Err(e) => {
                        status.set(format!("Error fetching {}: {}", name, e));
                    }
                }
            }
            
            mods.set(fetched_mods.clone());
            loading_mods.set(false);
            
            if fetched_mods.is_empty() {
                status.set("No mods loaded. Check your mod URLs.".to_string());
            } else {
                status.set(format!("Loaded {} mod(s) from Thunderstore", fetched_mods.len()));
            }
        });
    });

    let mut toggle_mod = move |mod_id: String| {
        mods.write().iter_mut().for_each(|m| {
            if m.id == mod_id {
                m.enabled = !m.enabled;
            }
        });
    };
    
    let select_all = move |_| {
        mods.write().iter_mut().for_each(|m| m.enabled = true);
    };
    
    let deselect_all = move |_| {
        mods.write().iter_mut().for_each(|m| m.enabled = false);
    };
    
    let download_to_steamapps = move |_| {
        if install_is_processing() {
            return; // Don't process if already processing
        }

        let selected_mods: Vec<Mod> = mods.read().iter()
            .filter(|m| m.enabled)
            .cloned()
            .collect();
        
        if selected_mods.is_empty() {
            status.set("Please select at least one mod to install".to_string());
            return;
        }

        if let Some(bepinex) = selected_mods.iter().find(|sel_mod| sel_mod.name == "BepInExPack") {
            let existing_valheim_dir = valheim_location();
            let bepinex_clone = bepinex.clone();
            spawn(async move {
                install_is_processing.set(true);
                let target_dir = PathBuf::from(existing_valheim_dir.clone().unwrap()).join(bepinex_clone.to.unwrap_or_default().clone());
                let mut installed_count = 0;
                let total_mods = selected_mods.len();

                match download_and_extract_mod(&bepinex_clone.download_url, bepinex_clone.from.clone(), &target_dir).await {
                    Ok(_) => {
                        installed_count += 1;
                        status.set(format!("Installed BepInEx: v{}", bepinex_clone.version));
                        
                        if existing_valheim_dir.is_none() {
                            status.set("Valheim installation not found when trying to install mods.".to_string());
                            return;
                        }

                        let valheim_exe = target_dir.join("valheim.exe");
                        status.set("Starting Valheim with BepInEx".to_string());
                        let valheim_proc = process::Command::new(valheim_exe).spawn();
                        match valheim_proc {
                            Ok(mut child) => {
                                let plugins_dir = target_dir.join("BepInEx").join("plugins");
                                let max_time = std::time::Duration::from_secs(300);
                                let start = std::time::Instant::now();
                                while !plugins_dir.is_dir() {
                                    if std::time::Instant::now() - start > max_time {
                                        break;
                                    }
                                    std::thread::sleep(time::Duration::from_secs(1));
                                }
                                
                                match child.kill() {
                                    Ok(..) => { status.set("Successfully closed Valheim".to_string()); }
                                    Err(e) => { status.set(format!("Error trying to close Valheim: {}", e)); }
                                }
                            }
                            Err(e) => { 
                                status.set(format!("Error starting Valheim with BepInEx: {}", e)); 
                            }
                        }
                        
                        // Download and extract each mod after bepinex
                        for mod_item in selected_mods {
                            if mod_item.name == "BepInExPack" { 
                                continue;
                            }

                            status.set(format!("Downloading {}/{}: {} v{}...", installed_count + 1, total_mods, mod_item.name, mod_item.version));

                            let internal_from_dir = mod_item.from;
                            let target_dir = PathBuf::from(existing_valheim_dir.clone().unwrap()).join(mod_item.to.unwrap_or_default());
                            
                            match download_and_extract_mod(&mod_item.download_url, internal_from_dir, &target_dir).await {
                                Ok(_) => {
                                    installed_count += 1;
                                    status.set(format!("Installed {}/{}: {} v{}", installed_count, total_mods, mod_item.name, mod_item.version));
                                }
                                Err(e) => {
                                    status.set(format!("Error installing {}: {}", mod_item.name, e));
                                    return;
                                }
                            }
                        }
                        
                        status.set(format!("Installation complete! {} mod(s) installed successfully.", installed_count));
                    }
                    Err(e) => {
                        status.set(format!("Error installing BepInEx: {}", e));
                        return;
                    }
                }
                install_is_processing.set(false);
            });
        }
    };
    
    let enabled_count = mods.read().iter().filter(|m| m.enabled).count();
    let primary_style = if primary_pressed() {
        "flex: 7; padding: 15px 30px; font-size: 16px; background-color: #0056b3; color: white; border: none; border-radius: 5px; cursor: pointer; transition: all 0.1s ease; transform: scale(0.95); box-shadow: inset 0 2px 4px rgba(0,0,0,0.2);"
    } else {
        "flex: 7; padding: 15px 30px; font-size: 16px; background-color: #007bff; color: white; border: none; border-radius: 5px; cursor: pointer; transition: all 0.1s ease; transform: scale(1);"
    };

    let secondary_style = if secondary_pressed() {
        "flex: 3; padding: 15px 30px; font-size: 16px; background-color: #545b62; color: white; border: none; border-radius: 5px; cursor: pointer; transition: all 0.1s ease; transform: scale(0.95); box-shadow: inset 0 2px 4px rgba(0,0,0,0.2);"
    } else {
        "flex: 3; padding: 15px 30px; font-size: 16px; background-color: #6c757d; color: white; border: none; border-radius: 5px; cursor: pointer; transition: all 0.1s ease; transform: scale(1);"
    };
    
    rsx! {
        div { 
            style: "padding: 40px; font-family: sans-serif; max-width: 800px; margin: 0 auto;",
            
            h1 { 
                style: "color: #1b2838; margin-bottom: 10px;",
                "Valheim Mod Installer" 
            }
            
            if auto_detect_failed() {
                div {
                    style: "background: #fff3cd; border: 1px solid #ffc107; padding: 15px; border-radius: 5px; margin: 20px 0;",
                    p { 
                        style: "margin: 0 0 10px 0;",
                        "Could not automatically detect Steam installation."
                    }
                    button {
                        style: "background: #1b2838; color: white; padding: 10px 20px; border: none; border-radius: 3px; cursor: pointer; font-size: 14px;",
                        onclick: select_valheim_directory,
                        "Select Steam Directory"
                    }
                }
            }

            div {
                style: "background: #f0f0f0; padding: 15px; border-radius: 5px; margin: 20px 0;",
                p { 
                    style: "margin: 0; font-size: 14px;",
                    strong { "Status: " }
                    "{status}"
                }
            }
            
            if valheim_location().is_some() {
                div {
                    style: "margin-top: 20px;",
                    p {
                        style: "color: #666; font-size: 14px; margin-bottom: 10px;",
                        "Steam Location: "
                        code { 
                            style: "background: #f5f5f5; padding: 2px 6px; border-radius: 3px;",
                            "{valheim_location().unwrap().display()}"
                        }
                    }
                    
                    if !mods.read().is_empty() {
                        div { 
                            style: "display: grid; grid-template-columns: 1fr 300px; gap: 20px; margin: 20px 0;",
                            div {
                                style: "background: white; border: 1px solid #ddd; border-radius: 5px; padding: 20px; margin: 20px 0;",
                                
                                div {
                                    style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 15px;",
                                    h2 {
                                        style: "margin: 0; font-size: 18px; color: #1b2838;",
                                        "Mods to Install ({enabled_count} selected)"
                                    }
                                    div {
                                        button {
                                            style: "background: #5c7e10; color: white; padding: 6px 12px; border: none; border-radius: 3px; cursor: pointer; font-size: 12px; margin-right: 5px;",
                                            onclick: select_all,
                                            "Select All"
                                        }
                                        button {
                                            style: "background: #666; color: white; padding: 6px 12px; border: none; border-radius: 3px; cursor: pointer; font-size: 12px;",
                                            onclick: deselect_all,
                                            "Deselect All"
                                        }
                                    }
                                }
                                
                                div {
                                    style: "display: flex; flex-direction: column; gap: 10px;",
                                    for mod_item in mods.read().iter() {
                                        div {
                                            key: "{mod_item.id}",
                                            style: "border: 1px solid #e0e0e0; border-radius: 4px; padding: 15px; background: #fafafa; cursor: pointer; transition: background 0.2s;",
                                            onclick: {
                                                let mod_id = mod_item.id.clone();
                                                move |_| toggle_mod(mod_id.clone())
                                            },
                                            
                                            div {
                                                style: "display: flex; align-items: start; gap: 15px;",
                                                input {
                                                    r#type: "checkbox",
                                                    checked: mod_item.enabled,
                                                    style: "margin-top: 2px; cursor: pointer; width: 18px; height: 18px; flex-shrink: 0;",
                                                }
                                                img {
                                                    src: "{mod_item.icon_url}",
                                                    style: "width: 64px; height: 64px; border-radius: 4px; object-fit: cover; flex-shrink: 0;",
                                                    alt: "{mod_item.name}"
                                                }
                                                div {
                                                    style: "flex: 1;",
                                                    h3 {
                                                        style: "margin: 0 0 5px 0; font-size: 16px; color: #1b2838;",
                                                        "{mod_item.name} "
                                                        span {
                                                            style: "font-size: 13px; color: #999; font-weight: normal;",
                                                            "v{mod_item.version}"
                                                        }
                                                    }
                                                    p {
                                                        style: "margin: 0; color: #666; font-size: 13px; line-height: 1.4;",
                                                        "{mod_item.description}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Right side - Install panel (sticky)
                            div {
                                style: "position: sticky; top: 20px; align-self: start;",
                                div {
                                    style: "display: flex; justify-content: center; align-items: center; min-height: 5vh; margin: 0; background-color: #f0f0f0; font-family: Arial, sans-serif;",
                    
                                    div {
                                        style: "display: flex; gap: 10px; width: 100%; max-width: 600px; padding: 10px",
                                        button {
                                            style: "{primary_style}",
                                            // style: "background: #5c7e10; color: white; padding: 12px 24px; border: none; border-radius: 3px; cursor: pointer; font-size: 16px; font-weight: bold; width: 100%;",
                                            disabled: install_is_processing() || uninstall_is_processing() || enabled_count == 0,
                                            onmousedown: move |_| primary_pressed.set(true),
                                            onmouseup: move |_| primary_pressed.set(false),
                                            onmouseleave: move |_| primary_pressed.set(false),
                                            onclick: download_to_steamapps,

                                            if install_is_processing() {
                                                Spinner {}
                                                " Processing..."
                                            } else {
                                                "Install Selected ({enabled_count})"
                                            }
                                        }

                                        button {
                                            // style: "background: #800000; color: white; padding: 10px 20px; border: none; border-radius: 3px; cursor: pointer; font-size: 14px;",
                                            style: "{secondary_style}",
                                            disabled: install_is_processing() || uninstall_is_processing(),
                                            onmousedown: move |_| secondary_pressed.set(true),
                                            onmouseup: move |_| secondary_pressed.set(false),
                                            onmouseleave: move |_| secondary_pressed.set(false),
                                            onclick: uninstall_mods,

                                            if uninstall_is_processing() {
                                                Spinner {}
                                                " Processing..."
                                            } else {
                                                "Uninstall"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn get_mods_json() -> Result<Vec<bepmod::BepinexMod>, Box<dyn std::error::Error>> {
    let response: Vec<bepmod::BepinexMod> = reqwest::get(MODS_JSON_URL).await?.json().await?;
    
    Ok(response)
}

#[component]
fn Spinner() -> Element {
    let mut rotation = use_signal(|| 0);
    
    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_millis(16)).await;
            rotation.set((rotation() + 6) % 360);
        }
    });
    
    rsx! {
        span {
            style: "display: inline-block; width: 16px; height: 16px; border: 2px solid rgba(255, 255, 255, 0.3); border-top: 2px solid white; border-radius: 50%; margin-right: 8px; vertical-align: middle; transform: rotate({rotation()}deg);",
        }
    }
}

async fn download_and_extract_mod(download_url: &str, from_dir: Option<String>, target_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Download the zip file
    let response = reqwest::get(download_url).await?;
    let bytes = response.bytes().await?;
    
    // Save to temporary file
    let temp_file = std::env::temp_dir().join("thunderstore_mod.zip");
    std::fs::write(&temp_file, bytes)?;
    
    // Extract the zip file
    let file = std::fs::File::open(&temp_file)?;
    let mut archive = zip::ZipArchive::new(file)?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_path = file.name().to_string();
        match from_dir.clone() {
            Some(internal_dir) => {
                if file_path.starts_with(&format!("{}/", internal_dir)) {
                    let relative_path = file_path.strip_prefix(&format!("{}/", internal_dir)).unwrap_or(&file_path);
                    let outpath = target_dir.join(relative_path);
                
                    if file.is_dir() {
                        std::fs::create_dir_all(&outpath)?;
                    } else {
                        if let Some(p) = outpath.parent() {
                            std::fs::create_dir_all(p)?;
                        }
                        let mut outfile = std::fs::File::create(&outpath)?;
                        std::io::copy(&mut file, &mut outfile)?;
                    }
                }
            },
            None => {
                let outpath = target_dir.join(file.name());
                
                if file.is_dir() {
                    std::fs::create_dir_all(&outpath)?;
                } else {
                    if let Some(p) = outpath.parent() {
                        std::fs::create_dir_all(p)?;
                    }
                    let mut outfile = std::fs::File::create(&outpath)?;
                    std::io::copy(&mut file, &mut outfile)?;
                }
            }
        }
    }
    
    // Clean up temp file
    std::fs::remove_file(&temp_file)?;
    
    Ok(())
}

fn find_steam_directory() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // First, try reading from Windows registry (most reliable)
        if let Some(path) = find_steam_from_registry() {
            return Some(path);
        }
        
        // Fallback to common paths
        let paths = vec![
            PathBuf::from("C:\\Program Files (x86)\\Steam"),
            PathBuf::from("C:\\Program Files\\Steam"),
        ];
        
        for path in paths {
            if path.join("steamapps").exists() {
                return Some(path);
            }
        }
        
        // Try all drives (C through Z)
        for drive in 'C'..='Z' {
            let path = PathBuf::from(format!("{}:\\Steam", drive));
            if path.join("steamapps").exists() {
                return Some(path);
            }
            let path = PathBuf::from(format!("{}:\\Program Files (x86)\\Steam", drive));
            if path.join("steamapps").exists() {
                return Some(path);
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let path = PathBuf::from(home).join("Library/Application Support/Steam");
            if path.join("steamapps").exists() {
                return Some(path);
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let possible_paths = vec![
                PathBuf::from(&home).join(".steam/steam"),
                PathBuf::from(&home).join(".local/share/Steam"),
                PathBuf::from(&home).join(".var/app/com.valvesoftware.Steam/.local/share/Steam"), // Flatpak
            ];
            
            for path in possible_paths {
                if path.join("steamapps").exists() {
                    return Some(path);
                }
            }
        }
    }
    
    None
}

fn uninstall(valheim_path: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    match valheim_path {
        Some(valheim_path) => {
            let bepinex_dir = valheim_path.join("BepInEx");
            let doorstop_dir = valheim_path.join("doorstop_libs");
            let changelog = valheim_path.join("changelog.txt");
            let doorstop_config = valheim_path.join("doorstop_config.ini");
            let doorstop_version = valheim_path.join(".doorstop_version");
            let start_game_bepinex = valheim_path.join("start_game_bepinex.sh");
            let start_server_bepinex = valheim_path.join("start_server_bepinex.sh");
            let winhttp_dll = valheim_path.join("winhttp.dll");

            if bepinex_dir.is_dir() { std::fs::remove_dir_all(bepinex_dir)?; }
            if doorstop_dir.is_dir() { std::fs::remove_dir_all(doorstop_dir)?; }
            if changelog.is_file() { std::fs::remove_file(changelog)?; }
            if doorstop_config.is_file() { std::fs::remove_file(doorstop_config)?; }
            if doorstop_version.is_file() { std::fs::remove_file(doorstop_version)?; }
            if start_game_bepinex.is_file() { std::fs::remove_file(start_game_bepinex)?; }
            if start_server_bepinex.is_file() { std::fs::remove_file(start_server_bepinex)?; }
            if winhttp_dll.is_file() { std::fs::remove_file(winhttp_dll)?; }
        }
        None => {}
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn find_steam_from_registry() -> Option<PathBuf> {
    use std::process::Command;
    
    // Use reg.exe to query registry (works without winreg dependency)
    let output = Command::new("reg")
        .args(&[
            "query",
            "HKLM\\SOFTWARE\\WOW6432Node\\Valve\\Steam",
            "/v",
            "InstallPath"
        ])
        .output()
        .ok()?;
    
    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        // Parse output: "    InstallPath    REG_SZ    C:\Program Files (x86)\Steam"
        for line in output_str.lines() {
            if line.contains("InstallPath") && line.contains("REG_SZ") {
                if let Some(path_str) = line.split("REG_SZ").nth(1) {
                    let path = PathBuf::from(path_str.trim());
                    if path.join("steamapps").exists() {
                        return Some(path);
                    }
                }
            }
        }
    }
    
    // Try 32-bit registry key as well
    let output = Command::new("reg")
        .args(&[
            "query",
            "HKLM\\SOFTWARE\\Valve\\Steam",
            "/v",
            "InstallPath"
        ])
        .output()
        .ok()?;
    
    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.contains("InstallPath") && line.contains("REG_SZ") {
                if let Some(path_str) = line.split("REG_SZ").nth(1) {
                    let path = PathBuf::from(path_str.trim());
                    if path.join("steamapps").exists() {
                        return Some(path);
                    }
                }
            }
        }
    }
    
    None
}

// Find all Steam library folders (including additional libraries on other drives)
fn find_all_steam_libraries(steam_dir: &PathBuf) -> Vec<PathBuf> {
    let mut libraries = vec![steam_dir.clone()];
    
    let library_folders_path = steam_dir
        .join("steamapps")
        .join("libraryfolders.vdf");
    
    if let Ok(content) = std::fs::read_to_string(library_folders_path) {
        // Parse VDF format to extract library paths
        // VDF format example:
        // "libraryfolders"
        // {
        //     "0" { "path" "C:\\Program Files (x86)\\Steam" }
        //     "1" { "path" "D:\\SteamLibrary" }
        // }
        
        let mut in_path_value = false;
        for line in content.lines() {
            let trimmed = line.trim();
            
            // Look for "path" key
            if trimmed.starts_with("\"path\"") {
                in_path_value = true;
            }
            
            if in_path_value {
                // Extract the path value (between quotes)
                if let Some(start) = trimmed.find("\"path\"") {
                    let after_key = &trimmed[start + 6..].trim();
                    if let Some(path_start) = after_key.find('"') {
                        let path_str = &after_key[path_start + 1..];
                        if let Some(path_end) = path_str.find('"') {
                            let path = PathBuf::from(&path_str[..path_end].replace("\\\\", "\\"));
                            if path.exists() && path.join("steamapps").exists() {
                                libraries.push(path);
                            }
                        }
                    }
                }
                in_path_value = false;
            }
        }
    }
    
    libraries
}

// Find a specific game's installation directory across all Steam libraries
fn find_game_directory(game_folder_name: &str) -> Option<PathBuf> {
    let Some(steam_dir) = find_steam_directory() else { return None; };
    let libraries = find_all_steam_libraries(&steam_dir);
    
    for library in libraries {
        let game_path = library
            .join("steamapps")
            .join("common")
            .join(game_folder_name);
        
        if game_path.exists() {
            return Some(game_path);
        }
    }
    
    None
}

fn open_directory_picker() -> Option<PathBuf> {
    use rfd::FileDialog;
    
    FileDialog::new()
        .set_title("Select Valheim Directory")
        .pick_folder()
}