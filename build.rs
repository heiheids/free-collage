// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 黑山大叔 (Heishan Dashu / heiheids)

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let profile = out
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("target"));

    println!("cargo:rerun-if-changed=assets/app.ico");
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-changed=languages");
    println!("cargo:rerun-if-changed=build.rs");

    // Keep external language files and the icon next to the executable.
    copy_dir(&root.join("languages"), &profile.join("languages"));
    let icon_dir = profile.join("assets");
    let _ = fs::create_dir_all(&icon_dir);
    let _ = fs::copy(root.join("assets/app.ico"), icon_dir.join("app.ico"));

    if let Some(rc) = find_rc() {
        let res = out.join("app.res");
        let status = Command::new(&rc)
            .current_dir(root.join("assets"))
            .arg("/nologo")
            .arg("/fo")
            .arg(&res)
            .arg("/I")
            .arg(root.join("assets"))
            .arg("app.rc")
            .status();
        match status {
            Ok(s) if s.success() => {
                println!("cargo:rustc-link-arg={}", res.display());
            }
            Ok(s) => println!("cargo:warning=rc.exe failed with status {s}"),
            Err(e) => println!("cargo:warning=could not run rc.exe: {e}"),
        }
    } else {
        println!("cargo:warning=Windows SDK rc.exe not found; EXE icon metadata will be missing");
    }
}

fn copy_dir(from: &Path, to: &Path) {
    if !from.is_dir() {
        return;
    }
    let _ = fs::create_dir_all(to);
    let Ok(entries) = fs::read_dir(from) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let _ = fs::copy(&path, to.join(entry.file_name()));
        }
    }
}

fn find_rc() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("RC") {
        let p = PathBuf::from(path);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let p = dir.join("rc.exe");
            if p.is_file() {
                return Some(p);
            }
        }
    }
    let mut roots = Vec::new();
    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        roots.push(PathBuf::from(pf86).join("Windows Kits/10/bin"));
    }
    if let Ok(pf) = std::env::var("ProgramFiles") {
        roots.push(PathBuf::from(pf).join("Windows Kits/10/bin"));
    }
    for root in roots {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        let mut versions: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path().join("x64/rc.exe"))
            .filter(|p| p.is_file())
            .collect();
        versions.sort();
        if let Some(p) = versions.pop() {
            return Some(p);
        }
    }
    None
}
