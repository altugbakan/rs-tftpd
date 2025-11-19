use std::process::Command;

fn main() {
    let result = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();
    let mut git_hash = if let Ok(output) = result {
        String::from_utf8(output.stdout).unwrap_or(String::from("Non-UTF git output!? "))
    } else {
        String::from("Error calling 'git' ")
    };
    git_hash.pop();

    let result = Command::new("git").args(["status"]).output();
    let dirty = if let Ok(output) = result {
        if !String::from_utf8(output.stdout)
            .unwrap_or_default()
            .contains("modified:")
        {
            ""
        } else {
            "+"
        }
    } else {
        " Error calling 'git'"
    };
    println!(
        "cargo:rustc-env=GIT_HASH={}{}",
        git_hash,
        String::from(dirty)
    );

    let result = Command::new("date").output();
    let date = if let Ok(output) = result {
        String::from_utf8(output.stdout).unwrap_or(String::from("Non-UTF date output!? "))
    } else {
        String::from("Error calling 'date' ")
    };
    println!("cargo:rustc-env=BUILD_DATE={}", date);
}
