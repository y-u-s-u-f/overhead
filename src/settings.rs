use crate::location::{self, Coords};
use std::path::PathBuf;

fn path(name: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library/Application Support/Overhead")
            .join(name),
    )
}

fn read(name: &str) -> Option<String> {
    std::fs::read_to_string(path(name)?).ok()
}

fn write(name: &str, value: String) {
    let Some(path) = path(name) else { return };
    let result = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|_| std::fs::write(&path, value));
    if let Err(err) = result {
        eprintln!("couldn't save {name}: {err}");
    }
}

pub fn load_radius() -> Option<u32> {
    read("radius")?.trim().parse().ok()
}

pub fn save_radius(radius_mi: u32) {
    write("radius", radius_mi.to_string());
}

pub fn load_home() -> Option<Coords> {
    location::parse_coords(&read("home")?)
}

pub fn save_home((lat, lon): Coords) {
    write("home", format!("{lat}, {lon}"));
}
