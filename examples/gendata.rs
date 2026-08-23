//! This program is mainly intended for generating the dumps that are compiled in to
//! syntect, not as a helpful example for beginners.
//! Although it is a valid example for serializing syntaxes, you probably won't need
//! to do this yourself unless you want to cache your own compiled grammars.
//!
//! An example of how this script is used to generate the pack files included
//! with syntect can be found under `make packs` in the Makefile.
//!
//! ## Customizing Default Syntaxes and Themes
//!
//! Edit `examples/gendata/config.rs` to filter which syntaxes and themes are included
//! in the default packdumps. By default, all syntaxes and themes are included.

#[path = "gendata/config.rs"]
mod build_config;

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use syntect::dumps::*;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSetBuilder;

fn usage_and_exit() -> ! {
    println!(
        "USAGE: gendata synpack source-dir \
              newlines.packdump nonewlines.packdump \
              [metadata.packdump] [metadata extra-source-dir]\n       \
              gendata themepack source-dir themepack.themedump\n       \
              gendata dictgen source-dir zstd.dict lz4.dict"
    );
    ::std::process::exit(2);
}

/// Add syntaxes from a folder, applying the build_config filter.
fn add_syntaxes_filtered(builder: &mut SyntaxSetBuilder, folder: &str, lines_include_newline: bool) {
    let folder_path = Path::new(folder);

    if build_config::SYNTAX_FILTER.is_none() {
        // No filter - use the original behavior for efficiency
        builder.add_from_folder(folder, lines_include_newline).unwrap();
        return;
    }

    // Filter is set - iterate through subfolders and add only matching ones
    let mut included = Vec::new();
    let mut excluded = Vec::new();

    for entry in std::fs::read_dir(folder_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_dir() {
            let name = path.file_name().unwrap().to_str().unwrap();
            if build_config::should_include_syntax(name) {
                builder.add_from_folder(&path, lines_include_newline).unwrap();
                included.push(name.to_string());
            } else {
                excluded.push(name.to_string());
            }
        } else if path.extension().map_or(false, |ext| ext == "sublime-syntax" || ext == "tmLanguage") {
            // Handle loose syntax files in the root
            let name = path.file_stem().unwrap().to_str().unwrap();
            if build_config::should_include_syntax(name) {
                builder.add_from_folder(&path, lines_include_newline).ok();
                included.push(name.to_string());
            }
        }
    }

    println!("Syntax filter applied:");
    println!("  Included {} syntaxes: {:?}", included.len(), included);
    if !excluded.is_empty() {
        println!("  Excluded {} syntaxes", excluded.len());
    }
}

/// Load themes from a folder, applying the build_config filter.
fn load_themes_filtered(folder: &str) -> ThemeSet {
    if build_config::THEME_FILTER.is_none() {
        // No filter - use the original behavior
        return ThemeSet::load_from_folder(folder).unwrap();
    }

    // Filter is set - load themes and filter them
    let mut ts = ThemeSet::load_from_folder(folder).unwrap();
    let original_count = ts.themes.len();

    let mut included = Vec::new();
    let mut excluded = Vec::new();

    ts.themes.retain(|name, _| {
        if build_config::should_include_theme(name) {
            included.push(name.clone());
            true
        } else {
            excluded.push(name.clone());
            false
        }
    });

    println!("Theme filter applied:");
    println!("  Included {} themes: {:?}", included.len(), included);
    println!("  Excluded {} themes (from {} total)", excluded.len(), original_count);

    ts
}

/// Extract raw (uncompressed) samples from a built SyntaxSet for dictionary training.
/// Returns the serialized LazyContexts data for each syntax.
fn extract_training_samples(package_dir: &str) -> Vec<Vec<u8>> {
    // Build a SyntaxSet - the lazy contexts will be compressed
    let mut builder = SyntaxSetBuilder::new();
    builder.add_plain_text_syntax();
    builder.add_from_folder(package_dir, false).unwrap();
    let ss = builder.build();

    // For each syntax, we need the raw serialized LazyContexts
    // Since they're already compressed in serialized_lazy_contexts,
    // we decompress them to get the raw bitcode data
    let mut samples = Vec::new();
    for syntax in ss.syntaxes() {
        // The serialized_lazy_contexts contains compressed data
        // We decompress it to get raw samples for training
        if let Ok(raw) = syntect::compression::decompress(syntax.serialized_contexts()) {
            if !raw.is_empty() {
                samples.push(raw);
            }
        }
    }

    println!(
        "Extracted {} samples for dictionary training",
        samples.len()
    );
    samples
}

fn main() {
    let mut a = env::args().skip(1);
    match (a.next(), a.next(), a.next(), a.next(), a.next(), a.next()) {
        (
            Some(ref cmd),
            Some(ref package_dir),
            Some(ref packpath_newlines),
            Some(ref packpath_nonewlines),
            ref _option_metapath,
            ref _option_metasource,
        ) if cmd == "synpack" => {
            let mut builder = SyntaxSetBuilder::new();
            builder.add_plain_text_syntax();
            add_syntaxes_filtered(&mut builder, package_dir, true);
            let ss = builder.build();
            dump_to_uncompressed_file(&ss, packpath_newlines).unwrap();

            let mut builder_nonewlines = SyntaxSetBuilder::new();
            builder_nonewlines.add_plain_text_syntax();
            add_syntaxes_filtered(&mut builder_nonewlines, package_dir, false);

            #[cfg(feature = "metadata")]
            {
                if let Some(metasource) = _option_metasource {
                    // Metadata source is not filtered (it's for additional metadata only)
                    builder_nonewlines
                        .add_from_folder(metasource, false)
                        .unwrap();
                }
            }

            let ss_nonewlines = builder_nonewlines.build();
            dump_to_uncompressed_file(&ss_nonewlines, packpath_nonewlines).unwrap();

            #[cfg(feature = "metadata")]
            {
                if let Some(metapath) = _option_metapath {
                    dump_to_file(&ss_nonewlines.metadata(), metapath).unwrap();
                }
            }
        }
        (Some(ref s), Some(ref theme_dir), Some(ref packpath), ..) if s == "themepack" => {
            let ts = load_themes_filtered(theme_dir);
            dump_to_file(&ts, packpath).unwrap();
        }
        (Some(ref cmd), Some(ref package_dir), Some(ref zstd_path), Some(ref lz4_path), ..)
            if cmd == "dictgen" =>
        {
            let samples = extract_training_samples(package_dir);

            // Train and save zstd dictionary
            #[cfg(feature = "compression-zstd")]
            {
                println!("Training zstd dictionary...");
                let dict_size = 64 * 1024; // 64KB dictionary
                match syntect::compression::train_zstd_dictionary(&samples, dict_size) {
                    Ok(dict) => {
                        let mut f = File::create(zstd_path).unwrap();
                        f.write_all(&dict).unwrap();
                        println!(
                            "Saved zstd dictionary to {} ({} bytes)",
                            zstd_path,
                            dict.len()
                        );
                    }
                    Err(e) => {
                        eprintln!("Failed to train zstd dictionary: {}", e);
                    }
                }
            }
            #[cfg(not(feature = "compression-zstd"))]
            {
                println!("Skipping zstd dictionary (compression-zstd feature not enabled)");
                let _ = zstd_path;
            }

            // Train and save lz4 dictionary
            #[cfg(feature = "compression-lz4")]
            {
                println!("Training lz4 dictionary...");
                let dict_size = 64 * 1024; // 64KB dictionary
                let dict = syntect::compression::train_lz4_dictionary(&samples, dict_size);
                let mut f = File::create(lz4_path).unwrap();
                f.write_all(&dict).unwrap();
                println!(
                    "Saved lz4 dictionary to {} ({} bytes)",
                    lz4_path,
                    dict.len()
                );
            }
            #[cfg(not(feature = "compression-lz4"))]
            {
                println!("Skipping lz4 dictionary (compression-lz4 feature not enabled)");
                let _ = lz4_path;
            }
        }
        _ => usage_and_exit(),
    }
}
