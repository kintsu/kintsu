/// Normalize a package name from import format (abc_foo) to manifest format (abc-foo)
pub fn normalize_import_to_package_name(import_name: &str) -> String {
    import_name.replace('_', "-")
}

/// Normalize a package name from manifest format (abc-foo) to import format (abc_foo)
pub fn normalize_package_to_import_name(package_name: &str) -> String {
    package_name.replace('-', "_")
}
