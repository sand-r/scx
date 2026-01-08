// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This software may be used and distributed according to the terms of the
// GNU General Public License version 2.

use std::env;
use std::fs::{self, File};
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use scx_cargo::ClangInfo;
use vergen::EmitBuilder;

fn find_dot_git(start: &Path) -> Option<PathBuf> {
    for dir in start.ancestors() {
        let dot_git = dir.join(".git");
        if dot_git.exists() {
            return Some(dot_git);
        }
    }
    None
}

fn get_git_dirs(manifest_dir: &Path) -> Option<(PathBuf, PathBuf)> {
    let dot_git = find_dot_git(manifest_dir)?;

    if dot_git.is_dir() {
        return Some((dot_git.clone(), dot_git));
    }

    if !dot_git.is_file() {
        return None;
    }

    let dot_git_content = fs::read_to_string(&dot_git).ok()?;
    let git_dir_spec = dot_git_content.trim().strip_prefix("gitdir:")?.trim();
    let mut git_dir = PathBuf::from(git_dir_spec);
    if git_dir.is_relative() {
        git_dir = dot_git.parent()?.join(git_dir);
    }

    // In git worktrees, git_dir points to a worktree-specific directory which
    // may contain a 'commondir' file that points at the shared git dir
    // containing refs/packed-refs.
    let common_dir = if let Ok(common_dir_spec) = fs::read_to_string(git_dir.join("commondir")) {
        let mut common_dir = PathBuf::from(common_dir_spec.trim());
        if common_dir.is_relative() {
            common_dir = git_dir.join(common_dir);
        }
        common_dir
    } else {
        git_dir.clone()
    };

    Some((git_dir, common_dir))
}

fn emit_git_rerun_triggers() {
    let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") else {
        return;
    };
    let Some((git_dir, common_dir)) = get_git_dirs(Path::new(&manifest_dir)) else {
        return;
    };

    // Always rerun when the checked-out commit changes.
    println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());

    // Dirty state changes may update the index (e.g. staging changes).
    println!("cargo:rerun-if-changed={}", git_dir.join("index").display());

    // Some repos store refs in packed-refs rather than loose ref files.
    println!(
        "cargo:rerun-if-changed={}",
        common_dir.join("packed-refs").display()
    );

    // If HEAD points to a ref, watch the corresponding ref file too.
    if let Ok(head) = fs::read_to_string(git_dir.join("HEAD")) {
        if let Some(ref_path) = head.trim().strip_prefix("ref: ") {
            println!(
                "cargo:rerun-if-changed={}",
                common_dir.join(ref_path.trim()).display()
            );
        }
    }
}

fn gen_bindings() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let clang = ClangInfo::new().unwrap();
    let kernel_target = clang.kernel_target().unwrap();

    let mut vmlinux_tar_zst = File::open("vmlinux.tar.zst").unwrap();

    let mut vmlinux_h = String::new();

    // vmlinux.h is a symlink. dereference it here.
    let search: PathBuf = format!("vmlinux/arch/{kernel_target}/vmlinux.h").into();

    let mut vmlinux_tar = ruzstd::decoding::StreamingDecoder::new(&mut vmlinux_tar_zst).unwrap();
    let mut archive = tar::Archive::new(&mut vmlinux_tar);
    let vmlinux_link_entry = archive
        .entries()
        .unwrap()
        .find(|x| x.as_ref().unwrap().path().unwrap() == search.as_path())
        .unwrap()
        .unwrap();

    let vmlinux_path = PathBuf::from(vmlinux_link_entry.path().unwrap())
        .parent()
        .unwrap()
        .join(vmlinux_link_entry.link_name().unwrap().unwrap());

    vmlinux_tar_zst.rewind().unwrap();
    let vmlinux_tar = ruzstd::decoding::StreamingDecoder::new(&mut vmlinux_tar_zst).unwrap();

    tar::Archive::new(vmlinux_tar)
        .entries()
        .unwrap()
        .find(|x| x.as_ref().unwrap().path().unwrap() == vmlinux_path.as_path())
        .unwrap()
        .unwrap()
        .read_to_string(&mut vmlinux_h)
        .unwrap();

    let bindings = bindgen::Builder::default()
        .header_contents(&search.to_string_lossy(), &vmlinux_h)
        .allowlist_type("scx_exit_kind")
        .allowlist_type("scx_consts")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(PathBuf::from(&out_dir).join("bindings.rs"))
        .expect("Couldn't write bindings");
}

fn main() {
    emit_git_rerun_triggers();
    gen_bindings();

    EmitBuilder::builder()
        .git_sha(true)
        .git_dirty(true)
        .cargo_target_triple()
        .emit()
        .unwrap();

    let bindings = bindgen::Builder::default()
        .header("perf_wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .prepend_enum_name(false)
        .derive_default(true)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("perf_bindings.rs"))
        .expect("Couldn't write bindings!");
}
