use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    let workspace_root = find_workspace_root().unwrap_or_else(|| {
        eprintln!("error: run this tool from within the gloo workspace");
        process::exit(1);
    });

    let mut crates = discover_crates(&workspace_root);
    apply_changelog_status(&workspace_root.join("CHANGELOG.md"), &mut crates);

    if args.len() < 3 {
        print_status(&crates);
        if args.len() == 1 {
            eprintln!();
            eprintln!("Usage: what-if-i-bump <crate> <patch|minor|major>");
            eprintln!("Example: what-if-i-bump net minor");
        }
        return;
    }

    let target_short = &args[1];
    let bump_arg = &args[2];

    let target = resolve_crate(target_short, &crates);
    if target == "gloo" {
        eprintln!("error: cannot directly bump the root crate; bump a sub-crate instead");
        process::exit(1);
    }

    let bump = match bump_arg.as_str() {
        "patch" => BumpKind::Patch,
        "minor" => {
            if crates[&target].cargo_version.major == 0 {
                BumpKind::ZeroMinor
            } else {
                BumpKind::NonZeroMinor
            }
        }
        "major" => BumpKind::Major,
        other => {
            eprintln!(
                "error: unknown bump type '{}'. Use: patch, minor, major",
                other
            );
            process::exit(1);
        }
    };

    let reverse_deps = build_reverse_deps(&crates);
    let required = compute_cascade(&target, bump, &crates, &reverse_deps);

    print_plan(&target, &crates, &required);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

impl Version {
    fn parse(s: &str) -> Option<Self> {
        let s = s.trim().trim_matches('"');
        let mut parts = s.splitn(3, '.');
        Some(Self {
            major: parts.next()?.parse().ok()?,
            minor: parts.next()?.parse().ok()?,
            patch: parts.next()?.parse().ok()?,
        })
    }

    fn bumped(self, kind: BumpKind) -> Self {
        match kind {
            BumpKind::Level => self,
            BumpKind::Patch => Self {
                patch: self.patch + 1,
                ..self
            },
            BumpKind::ZeroMinor => Self {
                major: 0,
                minor: self.minor + 1,
                patch: 0,
            },
            BumpKind::NonZeroMinor => Self {
                minor: self.minor + 1,
                patch: 0,
                ..self
            },
            BumpKind::Major => Self {
                major: self.major + 1,
                minor: 0,
                patch: 0,
            },
        }
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum BumpKind {
    Level,
    Patch,
    NonZeroMinor,
    ZeroMinor,
    Major,
}

impl fmt::Display for BumpKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Level => "level",
            Self::Patch => "patch-bumping",
            Self::NonZeroMinor => "non-zero-minor-bumping",
            Self::ZeroMinor => "zero-minor-bumping",
            Self::Major => "major-bumping",
        })
    }
}

fn bump_kind_between(from: &Version, to: &Version) -> BumpKind {
    if to <= from {
        return BumpKind::Level;
    }
    if to.major > from.major {
        return BumpKind::Major;
    }
    if to.minor > from.minor {
        return if from.major == 0 {
            BumpKind::ZeroMinor
        } else {
            BumpKind::NonZeroMinor
        };
    }
    BumpKind::Patch
}

#[derive(Debug, Clone)]
struct CrateInfo {
    name: String,
    short_name: String,
    cargo_version: Version,
    changelog_version: Option<Version>,
    status: BumpKind,
    internal_deps: Vec<String>,
    is_macro_crate: bool,
    changelog_section: String,
}

fn find_workspace_root() -> Option<PathBuf> {
    let mut dir = env::current_dir().ok()?;
    loop {
        let cargo = dir.join("Cargo.toml");
        if cargo.exists() {
            let content = fs::read_to_string(&cargo).ok()?;
            if content.contains("[workspace]") {
                return Some(dir);
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn discover_crates(root: &Path) -> BTreeMap<String, CrateInfo> {
    let mut crates = BTreeMap::new();

    let root_toml: toml::Value = fs::read_to_string(root.join("Cargo.toml"))
        .expect("read root Cargo.toml")
        .parse()
        .expect("parse root Cargo.toml");

    let root_ver = Version::parse(root_toml["package"]["version"].as_str().unwrap()).unwrap();
    let root_deps = extract_gloo_deps(&root_toml);

    crates.insert(
        "gloo".into(),
        CrateInfo {
            name: "gloo".into(),
            short_name: "gloo".into(),
            cargo_version: root_ver,
            changelog_version: None,
            status: BumpKind::Level,
            internal_deps: root_deps,
            is_macro_crate: false,
            changelog_section: "gloo".into(),
        },
    );

    let crates_dir = root.join("crates");
    let mut entries: Vec<_> = fs::read_dir(&crates_dir)
        .expect("read crates/")
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let cargo_path = path.join("Cargo.toml");
        if !cargo_path.exists() {
            continue;
        }

        let toml_val: toml::Value = fs::read_to_string(&cargo_path)
            .unwrap_or_else(|_| panic!("read {}", cargo_path.display()))
            .parse()
            .unwrap_or_else(|_| panic!("parse {}", cargo_path.display()));

        let name = toml_val["package"]["name"].as_str().unwrap().to_string();
        let version = Version::parse(toml_val["package"]["version"].as_str().unwrap()).unwrap();
        let short = name.strip_prefix("gloo-").unwrap_or(&name).to_string();
        let deps = extract_gloo_deps(&toml_val);

        let is_macro = short.ends_with("-macros");
        let changelog_section = if is_macro {
            format!("gloo-{}", short.strip_suffix("-macros").unwrap())
        } else {
            name.clone()
        };

        crates.insert(
            name.clone(),
            CrateInfo {
                name,
                short_name: short,
                cargo_version: version,
                changelog_version: None,
                status: BumpKind::Level,
                internal_deps: deps,
                is_macro_crate: is_macro,
                changelog_section,
            },
        );
    }

    crates
}

fn extract_gloo_deps(toml_val: &toml::Value) -> Vec<String> {
    let mut deps = Vec::new();
    if let Some(table) = toml_val.get("dependencies").and_then(|d| d.as_table()) {
        for key in table.keys() {
            if key.starts_with("gloo-") {
                deps.push(key.clone());
            }
        }
    }
    deps.sort();
    deps
}

fn apply_changelog_status(path: &Path, crates: &mut BTreeMap<String, CrateInfo>) {
    let content = fs::read_to_string(path).expect("read CHANGELOG.md");
    let mut current_section: Option<String> = None;
    let mut found_version_for_section = false;

    for line in content.lines() {
        let t = line.trim();

        if let Some(rest) = t.strip_prefix("## `") {
            if let Some(name) = rest.strip_suffix('`') {
                current_section = Some(name.to_string());
                found_version_for_section = false;
                continue;
            }
        }

        if !found_version_for_section {
            if let Some(ref section) = current_section {
                if let Some(ver_str) = t.strip_prefix("### Version ") {
                    if let Some(ver) = Version::parse(ver_str) {
                        for info in crates.values_mut() {
                            if info.changelog_section == *section && !info.is_macro_crate {
                                info.changelog_version = Some(ver);
                                info.status = bump_kind_between(&info.cargo_version, &ver);
                            }
                        }
                        found_version_for_section = true;
                    }
                }
            }
        }
    }
}

fn resolve_crate(input: &str, crates: &BTreeMap<String, CrateInfo>) -> String {
    if crates.contains_key(input) {
        return input.to_string();
    }
    let full = format!("gloo-{}", input);
    if crates.contains_key(&full) {
        return full;
    }
    for info in crates.values() {
        if info.short_name == input {
            return info.name.clone();
        }
    }
    eprintln!("error: unknown crate '{}'", input);
    eprintln!("available crates:");
    for info in crates.values() {
        if info.name != "gloo" {
            eprintln!("  {} ({})", info.short_name, info.name);
        }
    }
    process::exit(1);
}

fn build_reverse_deps(crates: &BTreeMap<String, CrateInfo>) -> BTreeMap<String, Vec<String>> {
    let mut rev: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for info in crates.values() {
        for dep in &info.internal_deps {
            rev.entry(dep.clone()).or_default().push(info.name.clone());
        }
    }
    rev
}

fn compute_cascade(
    target: &str,
    bump: BumpKind,
    crates: &BTreeMap<String, CrateInfo>,
    reverse_deps: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, BumpKind> {
    let mut required: BTreeMap<String, BumpKind> = BTreeMap::new();
    required.insert(target.to_string(), bump);

    loop {
        let mut changed = false;
        let snapshot: Vec<_> = required.iter().map(|(k, &v)| (k.clone(), v)).collect();

        for (name, child_bump) in snapshot {
            if let Some(parents) = reverse_deps.get(&name) {
                for parent in parents {
                    if !crates.contains_key(parent) {
                        continue;
                    }
                    let cascade = child_bump;
                    let entry = required.entry(parent.clone()).or_insert(BumpKind::Level);
                    if cascade > *entry {
                        *entry = cascade;
                        changed = true;
                    }
                }
            }
        }

        if !changed {
            break;
        }
    }

    required
}

fn print_status(crates: &BTreeMap<String, CrateInfo>) {
    println!("Current crate status:");
    println!();

    let max_name = crates.values().map(|c| c.name.len()).max().unwrap_or(0);

    for info in crates.values() {
        if info.is_macro_crate {
            println!(
                "  {:<width$}  {:<10}  (companion of {})",
                info.name,
                info.cargo_version,
                info.changelog_section,
                width = max_name,
            );
            continue;
        }
        let status = match info.status {
            BumpKind::Level => "level".to_string(),
            _ => {
                let cv = info.changelog_version.unwrap();
                format!("{} (-> {})", info.status, cv)
            }
        };
        println!(
            "  {:<width$}  {:<10}  {}",
            info.name,
            info.cargo_version,
            status,
            width = max_name,
        );
    }
}

fn print_plan(
    target: &str,
    crates: &BTreeMap<String, CrateInfo>,
    required: &BTreeMap<String, BumpKind>,
) {
    let mut new_versions: BTreeMap<String, Version> = BTreeMap::new();
    for (name, &needed) in required {
        let info = &crates[name];
        let effective = std::cmp::max(needed, info.status);
        new_versions.insert(name.clone(), info.cargo_version.bumped(effective));
    }

    struct Action {
        name: String,
        cargo_ver: Version,
        current_status: BumpKind,
        effective_bump: BumpKind,
        new_version: Version,
        needs_change: bool,
        is_macro_crate: bool,
        changelog_section: String,
        dep_bumps: Vec<(String, Version)>,
    }

    let mut actions: Vec<Action> = Vec::new();

    for (name, &needed) in required {
        let info = &crates[name];
        let effective = std::cmp::max(needed, info.status);
        let new_ver = new_versions[name];
        let needs_change = effective > info.status;

        let mut dep_bumps = Vec::new();
        for dep in &info.internal_deps {
            if let Some(dep_ver) = new_versions.get(dep) {
                let dep_info = &crates[dep];
                dep_bumps.push((dep_info.name.clone(), *dep_ver));
            }
        }

        actions.push(Action {
            name: name.clone(),
            cargo_ver: info.cargo_version,
            current_status: info.status,
            effective_bump: effective,
            new_version: new_ver,
            needs_change,
            is_macro_crate: info.is_macro_crate,
            changelog_section: info.changelog_section.clone(),
            dep_bumps,
        });
    }

    actions.sort_by(|a, b| {
        let a_is_root = a.name == "gloo";
        let b_is_root = b.name == "gloo";
        let a_is_target = a.name == target;
        let b_is_target = b.name == target;
        // target first, then non-root alphabetically, root last
        if a_is_target != b_is_target {
            return b_is_target.cmp(&a_is_target);
        }
        if a_is_root != b_is_root {
            return a_is_root.cmp(&b_is_root);
        }
        a.name.cmp(&b.name)
    });

    // Print summary table
    println!("Bump cascade:");
    println!();

    let max_name = actions.iter().map(|a| a.name.len()).max().unwrap_or(0);

    for a in &actions {
        let current = if a.current_status == BumpKind::Level {
            format!("{}", a.cargo_ver)
        } else {
            let cl = crates[&a.name].changelog_version.unwrap();
            format!("{} (-> {})", a.cargo_ver, cl)
        };

        let arrow_and_target = if a.needs_change {
            format!("=>  {} ({})", a.new_version, a.effective_bump)
        } else {
            "    (already satisfied)".to_string()
        };

        let reason = if a.name == target {
            "(target)".to_string()
        } else if a.dep_bumps.is_empty() {
            String::new()
        } else {
            let deps: Vec<_> = a.dep_bumps.iter().map(|(n, _)| n.as_str()).collect();
            format!("[depends on {}]", deps.join(", "))
        };

        println!(
            "  {:<width$}  {:<20} {}  {}",
            a.name,
            current,
            arrow_and_target,
            reason,
            width = max_name,
        );
    }

    // Collect changelog edits needed
    let edits: Vec<_> = actions.iter().filter(|a| a.needs_change).collect();

    if edits.is_empty() {
        println!();
        println!("All required CHANGELOG sections already exist. Add your changes there.");
        return;
    }

    println!();
    println!("CHANGELOG.md edits:");
    println!();

    // Group by changelog section to merge macro crate bumps into parent section
    struct SectionEdit {
        section: String,
        new_version: Version,
        is_new_section: bool,
        old_version: Option<Version>,
        dep_lines: Vec<String>,
    }

    let mut section_edits: BTreeMap<String, SectionEdit> = BTreeMap::new();

    for a in &edits {
        let section = &a.changelog_section;
        let entry = section_edits
            .entry(section.clone())
            .or_insert_with(|| SectionEdit {
                section: section.clone(),
                new_version: a.new_version,
                is_new_section: a.current_status == BumpKind::Level,
                old_version: crates[&a.name].changelog_version,
                dep_lines: Vec::new(),
            });

        if a.is_macro_crate {
            entry
                .dep_lines
                .push(format!("- Bump `{}` to {}", a.name, a.new_version));
        } else {
            entry.new_version = a.new_version;
            entry.is_new_section = a.current_status == BumpKind::Level;
            entry.old_version = crates[&a.name].changelog_version;
            for (dep_name, dep_ver) in &a.dep_bumps {
                if !crates[dep_name].is_macro_crate {
                    entry
                        .dep_lines
                        .push(format!("- Bump `{}` to {}", dep_name, dep_ver));
                }
            }
        }
    }

    for edit in section_edits.values() {
        if edit.is_new_section {
            println!("  ## `{}`  -- add at top:", edit.section);
            println!();
            println!("    ### Version {}", edit.new_version);
            if !edit.dep_lines.is_empty() {
                println!();
                for line in &edit.dep_lines {
                    println!("    {}", line);
                }
            }
        } else {
            let old = edit.old_version.unwrap();
            println!(
                "  ## `{}`  -- rename top version: {} => {}",
                edit.section, old, edit.new_version
            );
            if !edit.dep_lines.is_empty() {
                println!();
                println!("    Also add:");
                for line in &edit.dep_lines {
                    println!("    {}", line);
                }
            }
        }
        println!();
    }
}
