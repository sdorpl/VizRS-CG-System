// Wstawia skrót commita do `vizrs --version` (bez zależności zewnętrznych).
// Bez katalogu .git (np. budowa w Dockerze) skrót można podać zmienną VIZRS_GIT_HASH.
fn main() {
    let hash = std::env::var("VIZRS_GIT_HASH")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            std::process::Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "dev".to_string());
    println!("cargo:rustc-env=VIZRS_GIT={hash}");
}
