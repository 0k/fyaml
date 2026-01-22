use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=README.org");
    println!("cargo:rerun-if-changed=README.md");
    println!("cargo:rerun-if-changed=CHANGELOG.md");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("README.md");

    // In packaged crate, README.org is excluded - copy pre-generated README.md to OUT_DIR
    if !PathBuf::from("README.org").exists() {
        fs::copy("README.md", &out_path).expect("copy README.md to OUT_DIR");
        return;
    }

    // Run .package.d script to generate README.md (handles Lua filter for rustdoc attrs)
    run_package_script();

    // Use generated README.md, strip changelog for rustdoc
    let out = if let Ok(readme) = fs::read_to_string("README.md") {
        strip_changelog(&readme)
    } else {
        // Fallback: minimal conversion if script failed
        let org = fs::read_to_string("README.org").expect("read README.org");
        minimal_org_to_md(&org)
    };

    fs::write(&out_path, out).expect("write README.md");
}

/// Run .package.d/autogen.d/20-org-to-md.sh to generate README.md
fn run_package_script() {
    use std::io::Write;
    use std::process::Stdio;

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let script_path = PathBuf::from(&manifest_dir).join(".package.d/autogen.d/20-org-to-md.sh");

    if !script_path.exists() {
        return;
    }

    // Read script content and prepend stub function
    let script_content = match fs::read_to_string(&script_path) {
        Ok(content) => content,
        Err(_) => return,
    };

    let full_script = format!("depends() {{ :; }}\n{}", script_content);

    // Run via stdin to avoid quoting issues
    let mut child = match Command::new("sh")
        .current_dir(&manifest_dir)
        .stdin(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return,
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(full_script.as_bytes());
    }

    let _ = child.wait();
}

/// Strip changelog section from README (everything from "Changelog\n===" onwards)
fn strip_changelog(readme: &str) -> String {
    if let Some(pos) = readme.find("\nChangelog\n") {
        readme[..pos].trim_end().to_string()
    } else {
        readme.to_string()
    }
}

/// Minimal org-to-markdown converter (code blocks only)
fn minimal_org_to_md(org: &str) -> String {
    let mut out = String::new();
    let mut in_block = false;

    for line in org.lines() {
        if let Some(rest) = line.strip_prefix("#+begin_src") {
            in_block = true;
            let lang = rest.trim();
            out.push_str("```");
            if !lang.is_empty() {
                out.push_str(lang);
            }
            out.push('\n');
        } else if line.trim() == "#+end_src" && in_block {
            in_block = false;
            out.push_str("```\n");
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }

    out
}
