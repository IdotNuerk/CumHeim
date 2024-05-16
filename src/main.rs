#![windows_subsystem = "windows"]
use windows::Win32::Storage::FileSystem;
use core::time;
use std::{fs, io, process};
use std::path::Path;
use ureq;
use std::io::{Read, Write};
use zip;
use tempfile::{self, NamedTempFile};
use eframe::egui;
use std::thread;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
mod settings;

const NUM_URLS: usize = 3;
const MODS_JSON_URL: &'static str = "https://raw.githubusercontent.com/IdotNuerk/CumHeim/master/mods.json";
const TOTAL_PROGRESS: usize = 4 + NUM_URLS;

static RUNNING: AtomicBool = AtomicBool::new(false);
static PROGRESS: AtomicU32 = AtomicU32::new(0);

fn main() {
    app().unwrap();
}

fn app() -> Result<(), eframe::Error> {

    let options = eframe::NativeOptions {
        viewport: egui:: ViewportBuilder::default().with_inner_size([330.0, 140.0]),
        ..Default::default()
    };

    eframe::run_simple_native("Cumheim Installer", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {

            ui.horizontal(|ui| {
                ui.style_mut().text_styles.insert(
                    egui::TextStyle::Button, 
                    egui::FontId::new(36.0, eframe::epaint::FontFamily::Proportional),
                );

                ui.vertical(|buttons_ui| {
                    let install_button = buttons_ui.add_enabled(!RUNNING.load(Ordering::Acquire), egui::Button::new("Install"));
                    let uninstall_button = buttons_ui.add_enabled(!RUNNING.load(Ordering::Acquire), egui::Button::new("Uninstall"));
    
                    let install_resp = install_button.interact(egui::Sense::click());
                    let uninstall_resp = uninstall_button.interact(egui::Sense::click());
                    if install_resp.clicked() {
                        RUNNING.store(true, Ordering::Release);
                        PROGRESS.store(0, Ordering::Release);
                        thread::spawn(|| {
                            match install() {
                                Ok(..) => {
                                    RUNNING.store(false, Ordering::Release);
                                }
                                Err(e) => { println!("{:?}", e) }
                            };
                        });
                    }
    
                    if uninstall_resp.clicked() {
                        RUNNING.store(true, Ordering::Release);
                        PROGRESS.store(0, Ordering::Release);
                        thread::spawn(|| {
                            if let Ok(found_dir) = locate_valheim() {
                                match uninstall(found_dir) {
                                    Ok(..) => {
                                        RUNNING.store(false, Ordering::Release);
                                    }
                                    Err(e) => { println!("{:?}", e) }
                                };

                                for _ in 0..3 + NUM_URLS {
                                    PROGRESS.store(PROGRESS.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                                }
                            };
                        });
                    }
                });
            });
        });

        let bottom_panel = egui::TopBottomPanel::bottom(egui::Id::new("BottomPanel"))
            .show_separator_line(false);
        bottom_panel.show(ctx, |ui| {
            ui.add(egui::ProgressBar::new(PROGRESS.load(Ordering::Acquire) as f32 / TOTAL_PROGRESS as f32).animate(RUNNING.load(Ordering::Acquire)).show_percentage());
        });
    })
}

fn locate_valheim() -> Result<Option<String>, io::Error> {
    // Locate valheim steamapp dir
    let mut found_dir = None;
    if Path::new("C:\\Program Files (x86)\\Steam\\steamapps\\common\\Valheim").is_dir() {
        found_dir = Some("C:\\Program Files (x86)\\Steam\\steamapps\\common\\Valheim".to_owned());
    }
    else if Path::new("C:\\Program Files\\Steam\\steamapps\\common\\Valheim").is_dir() {
        found_dir = Some("C:\\Program Files\\Steam\\steamapps\\common\\Valheim".to_owned());
    }
    else {
        let drives = get_drives();

        for d in drives {
            let steamapps_dir = find_steamapps(Path::new(&d), 0);
            match steamapps_dir {
                Some(steamapps) => { 
                    if Path::new(&(steamapps.clone() + "\\common\\Valheim")).is_dir() {
                        found_dir = Some(steamapps + "\\common\\Valheim");
                        break;
                    } 
                }
                None => {}
            }
        }
    }

    // Increment progress bar
    PROGRESS.store(PROGRESS.load(Ordering::Relaxed) + 1, Ordering::Relaxed);

    Ok(found_dir)
}

fn uninstall(found_dir: Option<String>) -> Result<(), io::Error> {
    match found_dir {
        Some(valheim) => {
            let valheim_path = Path::new(&valheim);
            let bepinex_dir = valheim_path.join("BepInEx");
            let doorstop_dir = valheim_path.join("doorstop_libs");
            let changelog = valheim_path.join("changelog.txt");
            let doorstop_config = valheim_path.join("doorstop_config.ini");
            let start_game_bepinex = valheim_path.join("start_game_bepinex.sh");
            let start_server_bepinex = valheim_path.join("start_server_bepinex.sh");
            let winhttp_dll = valheim_path.join("winhttp.dll");

            if bepinex_dir.is_dir() { std::fs::remove_dir_all(bepinex_dir)?; }
            if doorstop_dir.is_dir() { std::fs::remove_dir_all(doorstop_dir)?; }
            if changelog.is_file() { std::fs::remove_file(changelog)?; }
            if doorstop_config.is_file() { std::fs::remove_file(doorstop_config)?; }
            if start_game_bepinex.is_file() { std::fs::remove_file(start_game_bepinex)?; }
            if start_server_bepinex.is_file() { std::fs::remove_file(start_server_bepinex)?; }
            if winhttp_dll.is_file() { std::fs::remove_file(winhttp_dll)?; }
        }
        None => {}
    }

    Ok(())
}

fn install() -> Result<(), std::io::Error> {
    // Locate valheim steamapp dir
    let found_dir = locate_valheim()?;

    // Increment progress bar
    PROGRESS.store(PROGRESS.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
    
    // If we found Valheim
    match found_dir.clone() {
        Some(valheim) => {
            println!("Found {:?}", valheim);

            // Check if bepinex is already installed
            if Path::new(&valheim.clone()).join("BepInEx").is_dir() {
                uninstall(found_dir)?;
            }

            PROGRESS.store(PROGRESS.load(Ordering::Relaxed) + 1, Ordering::Relaxed);

            // Download mods.json from github
            let settings = get_mods_json()?;

            // Download and extract BepInEx 
            match download_zip(&settings.bepinex.url, vec!(settings.bepinex.mapping), &valheim.clone()) {
                Ok(..) => {}
                Err(e) => { println!("Error while downloading BepInEx: {:?}", e)}
            }

            // Run valheim and close to init plugins folder
            let valheim_exe = valheim.clone() + "\\valheim.exe";
            println!("Running {:?}", valheim_exe);
            let valheim_proc = process::Command::new(valheim_exe).spawn();

            PROGRESS.store(PROGRESS.load(Ordering::Relaxed) + 1, Ordering::Relaxed);

            match valheim_proc {
                Ok(mut child) => {
                    let bepinex_dir = Path::new(&valheim.clone()).join("BepInEx");
                    let max_time = std::time::Duration::from_secs(300);
                    let start = std::time::Instant::now();
                    while !bepinex_dir.is_dir() {
                        if std::time::Instant::now() - start > max_time {
                            break;
                        }
                        std::thread::sleep(time::Duration::from_secs(1));
                    }
                    
                    match child.kill() {
                        Ok(..) => {}
                        Err(e) => { println!("Error closing Valheim.exe: {:?}", e) }
                    }
                }
                Err(e) => { println!("Error while starting valheim.exe: {:?}", e) }
            }

            PROGRESS.store(PROGRESS.load(Ordering::Relaxed) + 1, Ordering::Relaxed);

            // Download the rest of the mods
            for m in settings.mods {
                match download_zip(&m.url, m.mapping, &valheim.clone()) {
                    Ok(..) => {}
                    Err(e) => { println!("{:?}", e) }
                }

                PROGRESS.store(PROGRESS.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
            }
        }
        None => { 
            println!("Valheim directory was not found. Exiting...");
            return Err(std::io::Error::new(io::ErrorKind::InvalidInput, "Valheim directory was not found. Exiting..."));
        }
    }

    Ok(())
}

fn get_mods_json() -> Result<settings::Settings, std::io::Error> {
    let response = match ureq::get(MODS_JSON_URL).call() {
        Ok(res) => res,
        Err(err) => {
            eprintln!("Error sending request: {}", err);
            return Err(std::io::Error::new(io::ErrorKind::Other, err));
        }
    };

    let settings: settings::Settings = response.into_json()?;
    
    Ok(settings)
}

fn download_zip(src_url: &str, move_from_to: Vec<Vec<String>>, base_dest_path: &str ) -> Result<(), std::io::Error> {
    let get_resp: Result<ureq::Response, ureq::Error> = ureq::get(src_url) 
        .set("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
        .set("Cache-Control", "no-cache")
        .call();

    match get_resp {
        Ok(resp) => {
            let len: usize = resp.header("Content-Length")
                .unwrap()
                .parse().unwrap();

            let mut content_bytes: Vec<u8> = Vec::with_capacity(len);
            resp.into_reader()
                .take(100_000_000)
                .read_to_end(&mut content_bytes)?;

            let mut temp_file = NamedTempFile::new()?;
            temp_file.write_all(&content_bytes)?;
            println!("Writing to '{}'.", temp_file.path().display());

            let temp_reopened = temp_file.reopen()?;
            let zip_dir_str = temp_file.path().to_str().unwrap();
            let zip_dir_str2 = zip_dir_str.to_owned() + "_zip";
            let zip_dir_path = Path::new(&zip_dir_str2);

            let mut zip = zip::ZipArchive::new(temp_reopened)?;
            zip.extract(zip_dir_path)?;
            println!("Extracting to '{}'.", zip_dir_path.display());

            for mapping in move_from_to {
                let from = mapping[0].clone();
                let to = mapping[1].clone();
                let dest = Path::new(base_dest_path).join(to);
                if let Err(e) = fs::create_dir_all(dest.clone()) {
                    println!("Error while creating dir to dest: {:?}. Error: {:?}", dest.clone(), e);
                    return Err(e);
                }

                if from == "*" {
                    copy_dir_all(zip_dir_path, dest.clone())?;
                    println!("Copying contents in {:?} to '{:?}'.", zip_dir_path.display(), dest.display());
                } else {
                    let src = Path::new(zip_dir_path).join(from);
                    if src.is_dir() {
                        copy_dir_all(src.clone(), dest.clone())?;
                        println!("Copying contents in {:?} to '{:?}'.", src.display(), dest.display());
                    } else {
                        let dest_file_str = dest.to_str().unwrap().to_string().to_owned() + "\\" + src.file_name().unwrap().to_str().unwrap();
                        let dest_file = Path::new(&dest_file_str);
                        fs::copy(src.clone(), dest_file)?;
                        println!("Copying {:?} to '{:?}'.", src.display(), dest_file.display());
                    }

                }
            }

            // Remove temps
            fs::remove_file(temp_file.path())?;
            println!("Removing temp zipped file {:?}", temp_file.path().display());

            fs::remove_dir_all(zip_dir_path)?;
            println!("Removing temp unzipped dir {:?}", zip_dir_path.display());
        }
        Err(e) => { return Err(std::io::Error::new(std::io::ErrorKind::ConnectionAborted, e)) }
    }

    Ok(())
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn get_drives() -> Vec<String> {
    let mut drive_roots: Vec<String> = Vec::new();
    unsafe {
        let drives_bitmask = FileSystem::GetLogicalDrives();
        for i in 0..26
        {
            if drives_bitmask & (1 << i) != 0
            {
                let drive_letter = (('A'.to_ascii_uppercase() as u8) + i) as char;
                drive_roots.push(format!("{}:\\", drive_letter));
            }
        }
    }
    drive_roots
}

fn find_steamapps(dir: &Path, depth: u32) -> Option<String> {
    if depth > 3 {
        return None;
    }
    else if depth == 1 {
        println!("Checking in {:?}...", dir.display())
    }

    if dir.is_dir() {
        let entries = fs::read_dir(dir);
        match entries {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(e) => {
                            let path = e.path();
                            let pathstr = path.to_str().unwrap().to_owned() + "\\steamapps";
                            let dircheck = Path::new(&pathstr);
                            if dircheck.is_dir() {
                                return Some(dircheck.to_str().unwrap().to_owned());
                            }
        
                            if path.is_dir() {
                                find_steamapps(&path, depth + 1);
                            }
                        }
                        Err(..) => { return None; }
                    }
                }
            }
            Err(..) => {}
        }
    }
    return None;
}