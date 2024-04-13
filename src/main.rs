use windows::Win32::Storage::FileSystem;
use core::time;
use std::{fs, io, process};
use std::path::Path;
use ureq;
use std::io::{Read, Write};
use zip;
use copy_dir;
use tempfile::{self, NamedTempFile};
use eframe::egui;
use std::thread;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

// TODO: Change to arr of tuple<url, local_path>
const BEP_IN_EX_URL: &'static str = "https://thunderstore.io/package/download/denikson/BepInExPack_Valheim/5.4.2202/";
const NUM_URLS: usize = 2;
const URLS: [(&'static str, &'static str); NUM_URLS] = [
    ("https://github.com/Mydayyy/Valheim-ServerSideMap/releases/download/v1.3.11/ServerSideMap.zip", "\\BepInEx\\plugins"),
    ("https://thunderstore.io/package/download/Smoothbrain/Jewelcrafting/1.5.19/", "\\BepInEx\\plugins"),
    ];
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
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Button, 
                egui::FontId::new(36.0, eframe::epaint::FontFamily::Proportional),
            );
            let install_button = ui.add_enabled(!RUNNING.load(Ordering::Acquire), egui::Button::new("Install"));
            let uninstall_button = ui.add_enabled(!RUNNING.load(Ordering::Acquire), egui::Button::new("Uninstall"));

            let install_resp = install_button.interact(egui::Sense::click());
            let uninstall_resp = uninstall_button.interact(egui::Sense::click());
            if install_resp.clicked() {
                RUNNING.store(true, Ordering::Release);
                thread::spawn(|| {
                    match install() {
                        Ok(..) => {}
                        Err(e) => { println!("{:?}", e) }
                    };
                });
            }

            if uninstall_resp.clicked() {
                RUNNING.store(true, Ordering::Release);
                thread::spawn(|| {
                    match uninstall() {
                        Ok(..) => {}
                        Err(e) => { println!("{:?}", e) }
                    };
                });
            }
        });

        let bottom_panel = egui::TopBottomPanel::bottom(egui::Id::new("BottomPanel"))
            .show_separator_line(false);
        bottom_panel.show(ctx, |ui| {
            ui.add(egui::ProgressBar::new(PROGRESS.load(Ordering::Acquire) as f32 / TOTAL_PROGRESS as f32).animate(RUNNING.load(Ordering::Acquire)).show_percentage());
        });
    })
}

fn uninstall() -> Result<(), io::Error> {
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

    match found_dir {
        Some(valheim) => {
            let valheim_backup_str = valheim.replace("\\Valheim", "\\.Valheim");
            let valheim_backup_path = Path::new(&valheim_backup_str);
            if valheim_backup_path.is_dir() {
                std::fs::remove_dir_all(valheim.clone())?;
                std::fs::rename(valheim_backup_path, Path::new(&valheim))?;
            }
        }
        None => {}
    }

    for _ in 0..3 + NUM_URLS {
        PROGRESS.store(PROGRESS.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
    }

    Ok(())
}

fn install() -> Result<(), std::io::Error> {
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
    
    // If we found Valheim
    match found_dir {
        Some(valheim) => {
            println!("Found {:?}", valheim);

            // Backup Valheim to .Valheim
            let valheim_backup_str = valheim.replace("\\Valheim", "\\.Valheim");
            let valheim_backup_path = Path::new(&valheim_backup_str);
            if valheim_backup_path.is_dir() {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, ".Valheim path already exists."));
            }

            match copy_dir::copy_dir(Path::new(&valheim), valheim_backup_path) {
                Ok(..) => {}
                Err(e) => { 
                    println!("Failed to back up. Error: {:?}", e); 
                    return Err(e); 
                }
            }

            PROGRESS.store(PROGRESS.load(Ordering::Relaxed) + 1, Ordering::Relaxed);

            // Download and extract BepInEx 
            match download_bepinex(valheim.clone()) {
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
                    std::thread::sleep(time::Duration::from_secs(30));
                    match child.kill() {
                        Ok(..) => {}
                        Err(e) => { println!("Error closing Valheim.exe: {:?}", e) }
                    }
                }
                Err(e) => { println!("Error while starting valheim.exe: {:?}", e) }
            }

            PROGRESS.store(PROGRESS.load(Ordering::Relaxed) + 1, Ordering::Relaxed);

            // Download the rest of the mods
            for url in URLS {
                // Get dest path
                let dest = valheim.clone() + url.1;
                if let Err(e) = fs::create_dir_all(dest.clone()) {
                    println!("Error while creating dir to dest: {:?}. Error: {:?}", dest.clone(), e);
                    return Err(e);
                }

                match download_zip(url.0, dest) {
                    Ok(..) => {}
                    Err(e) => { println!("{:?}", e) }
                }

                PROGRESS.store(PROGRESS.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
            }
        }
        None => { println!("Valheim directory was not found. Exiting...") }
    }

    Ok(())
}

fn download_bepinex(dest_dir_path: String) -> Result<(), std::io::Error>
{
    let temp_dir = tempfile::TempDir::new()?;
    download_zip(BEP_IN_EX_URL, temp_dir.path().to_str().unwrap().to_owned())?;

    let bepinex_path = temp_dir.path().join("BepInExPack_Valheim");
    println!("Copying contents in {:?} to '{:?}'.", bepinex_path.display(), dest_dir_path);
    copy_dir_all(bepinex_path, dest_dir_path)?;
    

    Ok(())
}

fn download_zip(src_url: &str, dest_dir_path: String) -> Result<(), std::io::Error>
{
    let get_resp: Result<ureq::Response, ureq::Error> = ureq::get(src_url) 
        .set("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
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

            let file_path = Path::new(&dest_dir_path);

            copy_dir_all(zip_dir_path, file_path)?;
            println!("Copying contents in {:?} to '{:?}'.", zip_dir_path.display(), file_path.display());

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

fn get_drives() -> Vec<String>
{
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

fn find_steamapps(dir: &Path, depth: u32) -> Option<String>
{
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