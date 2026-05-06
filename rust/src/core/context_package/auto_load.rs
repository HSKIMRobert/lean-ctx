use super::registry::LocalRegistry;

pub fn auto_load_packages(project_root: &str) -> Vec<String> {
    let Ok(registry) = LocalRegistry::open() else {
        return Vec::new();
    };

    let Ok(packages) = registry.auto_load_packages() else {
        return Vec::new();
    };

    let mut loaded = Vec::new();

    for entry in &packages {
        let Ok((manifest, content)) = registry.load_package(&entry.name, &entry.version) else {
            continue;
        };

        if super::loader::load_package(&manifest, &content, project_root).is_ok() {
            loaded.push(format!("{} v{}", entry.name, entry.version));
        }
    }

    loaded
}
