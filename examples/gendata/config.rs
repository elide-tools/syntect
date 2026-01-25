//! Build-time configuration for customizing default syntaxes and themes.
//!
//! This file (`examples/gendata/config.rs`) controls which languages and themes
//! are included in the default packdumps. To customize:
//!
//! 1. Modify the filter constants below
//! 2. Regenerate the packdumps:
//!
//! ```sh
//! cargo run --example gendata --features "..." -- synpack ...
//! cargo run --example gendata --features "..." -- themepack ...
//! ```
//!
//! Set a filter to `Some(&[...])` to include only the specified items.
//! Set a filter to `None` to include all items (default behavior).

/// Filter for which syntax packages to include.
///
/// If `Some`, only syntax packages whose folder name is in this list will be included.
/// If `None`, all syntax packages are included (default behavior).
///
/// Example - include only common web languages:
/// ```rust
/// pub const SYNTAX_FILTER: Option<&[&str]> = Some(&[
///     "JavaScript",
///     "TypeScript",
///     "HTML",
///     "CSS",
///     "JSON",
///     "Markdown",
///     "Python",
///     "Rust",
///     "Go",
///     "SQL",
///     "ShellScript",
///     "YAML",
/// ]);
/// ```
pub const SYNTAX_FILTER: Option<&[&str]> = Some(&[
    "JavaScript",
    "Python",
    "Java",
    "Kotlin",
    "TypeScript",
    "JSON",
    "YAML",
]);

/// Filter for which themes to include.
///
/// If `Some`, only themes whose name (from the .tmTheme filename) contains one of these
/// substrings will be included.
/// If `None`, all themes are included (default behavior).
///
/// Example - include only dark themes:
/// ```rust
/// pub const THEME_FILTER: Option<&[&str]> = Some(&[
///     "dark",
///     "Solarized (dark)",
///     "mocha",
///     "eighties",
///     "ocean",
/// ]);
/// ```
pub const THEME_FILTER: Option<&[&str]> = Some(&["Solarized (dark)"]);

/// Returns true if the syntax package name should be included based on the filter.
pub fn should_include_syntax(package_name: &str) -> bool {
    match SYNTAX_FILTER {
        None => true,
        Some(filter) => filter.iter().any(|&f| f == package_name),
    }
}

/// Returns true if the theme name should be included based on the filter.
pub fn should_include_theme(theme_name: &str) -> bool {
    match THEME_FILTER {
        None => true,
        Some(filter) => filter.iter().any(|&f| theme_name.contains(f)),
    }
}
